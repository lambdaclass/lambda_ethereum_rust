use std::{
    ffi::{CStr, CString},
    mem::MaybeUninit,
    path::{Path, PathBuf},
    ptr::{addr_of_mut, null_mut},
};

use crate::module::MLIRModule;
use crate::program::Program;
use crate::{context::Session, errors::CodegenError};
use llvm_sys::{
    core::{
        LLVMContextCreate, LLVMContextDispose, LLVMDisposeMessage, LLVMDisposeModule,
        LLVMPrintModuleToFile,
    },
    error::LLVMGetErrorMessage,
    target_machine::{
        LLVMCodeGenFileType, LLVMCodeGenOptLevel, LLVMCodeModel, LLVMCreateTargetMachine,
        LLVMDisposeTargetMachine, LLVMGetDefaultTargetTriple, LLVMGetHostCPUFeatures,
        LLVMGetHostCPUName, LLVMGetTargetFromTriple, LLVMRelocMode, LLVMTargetMachineEmitToFile,
        LLVMTargetRef,
    },
    transforms::pass_builder::{
        LLVMCreatePassBuilderOptions, LLVMDisposePassBuilderOptions, LLVMRunPasses,
    },
};
use mlir_sys::mlirTranslateModuleToLLVMIR;

use crate::context::Context;

pub mod context;
pub(crate) mod operations;
mod pass_manager;
pub use pass_manager::run_pass_manager;

pub fn compile(program: &Program, output_file: impl AsRef<Path>) -> Result<PathBuf, CodegenError> {
    let context = Context::new();
    let session = Session {
        raw_mlir_path: Some(output_file.as_ref().to_path_buf()),
        ..Default::default()
    };
    let mlir_module = context.compile(program, session)?;
    compile_to_object(&mlir_module, output_file)
}

/// Converts a module to an object.
/// The object will be written to the specified target path.
///
/// Returns the path to the object.
// TODO: pass options to the function
pub fn compile_to_object(
    module: &MLIRModule<'_>,
    output_file: impl AsRef<Path>,
) -> Result<PathBuf, CodegenError> {
    let target_file = output_file.as_ref().with_extension("o");

    // TODO: Rework so you can specify target and host features, etc.
    // Right now it compiles for the native cpu feature set and arch
    unsafe {
        let llvm_context = LLVMContextCreate();

        let op = module.melior_module.as_operation().to_raw();

        let llvm_module = mlirTranslateModuleToLLVMIR(op, llvm_context as *mut _) as *mut _;

        let mut null = null_mut();
        let mut error_buffer = addr_of_mut!(null);

        let target_triple = LLVMGetDefaultTargetTriple();

        let target_cpu = LLVMGetHostCPUName();

        let target_cpu_features = LLVMGetHostCPUFeatures();

        let mut target: MaybeUninit<LLVMTargetRef> = MaybeUninit::uninit();

        if LLVMGetTargetFromTriple(target_triple, target.as_mut_ptr(), error_buffer) != 0 {
            let error = CStr::from_ptr(*error_buffer);
            let err = error.to_string_lossy().to_string();
            LLVMDisposeMessage(*error_buffer);
            return Err(CodegenError::LLVMCompileError(err));
        } else if !(*error_buffer).is_null() {
            LLVMDisposeMessage(*error_buffer);
            error_buffer = addr_of_mut!(null);
        }

        let target = target.assume_init();

        let machine = LLVMCreateTargetMachine(
            target,
            target_triple.cast(),
            target_cpu.cast(),
            target_cpu_features.cast(),
            LLVMCodeGenOptLevel::LLVMCodeGenLevelNone,
            LLVMRelocMode::LLVMRelocPIC,
            LLVMCodeModel::LLVMCodeModelDefault,
        );

        let opts = LLVMCreatePassBuilderOptions();
        let opt = 0;
        let passes = CString::new(format!("default<O{opt}>")).unwrap();
        let error = LLVMRunPasses(llvm_module as *mut _, passes.as_ptr(), machine, opts);
        if !error.is_null() {
            let msg = LLVMGetErrorMessage(error);
            let msg = CStr::from_ptr(msg);
            return Err(CodegenError::LLVMCompileError(
                msg.to_string_lossy().into_owned(),
            ));
        }

        LLVMDisposePassBuilderOptions(opts);

        // Output the LLVM IR
        let filename = CString::new(
            target_file
                .with_extension("ll")
                .as_os_str()
                .to_string_lossy()
                .as_bytes(),
        )
        .unwrap();
        if LLVMPrintModuleToFile(llvm_module, filename.as_ptr(), error_buffer) != 0 {
            let error = CStr::from_ptr(*error_buffer);
            let err = error.to_string_lossy().to_string();
            LLVMDisposeMessage(*error_buffer);
            return Err(CodegenError::LLVMCompileError(err));
        } else if !(*error_buffer).is_null() {
            LLVMDisposeMessage(*error_buffer);
            error_buffer = addr_of_mut!(null);
        }

        // Output the object file
        let filename = CString::new(target_file.as_os_str().to_string_lossy().as_bytes()).unwrap();
        let ok = LLVMTargetMachineEmitToFile(
            machine,
            llvm_module,
            filename.as_ptr().cast_mut(),
            LLVMCodeGenFileType::LLVMObjectFile, // object (binary) or assembly (textual)
            error_buffer,
        );

        if ok != 0 {
            let error = CStr::from_ptr(*error_buffer);
            let err = error.to_string_lossy().to_string();
            LLVMDisposeMessage(*error_buffer);
            return Err(CodegenError::LLVMCompileError(err));
        } else if !(*error_buffer).is_null() {
            LLVMDisposeMessage(*error_buffer);
        }

        // Output the assembly
        let filename = CString::new(
            target_file
                .with_extension("asm")
                .as_os_str()
                .to_string_lossy()
                .as_bytes(),
        )
        .unwrap();
        let ok = LLVMTargetMachineEmitToFile(
            machine,
            llvm_module,
            filename.as_ptr().cast_mut(),
            LLVMCodeGenFileType::LLVMAssemblyFile,
            error_buffer,
        );

        if ok != 0 {
            let error = CStr::from_ptr(*error_buffer);
            let err = error.to_string_lossy().to_string();
            LLVMDisposeMessage(*error_buffer);
            return Err(CodegenError::LLVMCompileError(err));
        } else if !(*error_buffer).is_null() {
            LLVMDisposeMessage(*error_buffer);
        }

        LLVMDisposeTargetMachine(machine);
        LLVMDisposeModule(llvm_module);
        LLVMContextDispose(llvm_context);

        Ok(target_file)
    }
}

/// Links object file to produce an executable binary
// Taken from cairo_native
pub fn link_binary(
    objects: &[impl AsRef<Path>],
    output_filename: impl AsRef<Path>,
) -> std::io::Result<()> {
    let objects: Vec<_> = objects
        .iter()
        .map(|x| x.as_ref().display().to_string())
        .collect();
    let output_filename = output_filename.as_ref().to_string_lossy().to_string();

    let args: Vec<_> = {
        if cfg!(target_os = "macos") {
            let mut args = vec![
                "-L/usr/local/lib",
                "-L/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk/usr/lib",
            ];

            args.extend(objects.iter().map(|x| x.as_str()));

            args.extend(&["-o", &output_filename, "-lSystem"]);

            args
        } else if cfg!(target_os = "linux") {
            let (scrt1, crti, crtn) = {
                if Path::new("/usr/lib64/Scrt1.o").exists() {
                    (
                        "/usr/lib64/Scrt1.o",
                        "/usr/lib64/crti.o",
                        "/usr/lib64/crtn.o",
                    )
                } else {
                    (
                        "/lib/x86_64-linux-gnu/Scrt1.o",
                        "/lib/x86_64-linux-gnu/crti.o",
                        "/lib/x86_64-linux-gnu/crtn.o",
                    )
                }
            };

            let mut args = vec![
                "-pie",
                "--hash-style=gnu",
                "--eh-frame-hdr",
                "--dynamic-linker",
                "/lib/x86_64-linux-gnu/ld-linux-x86-64.so.2",
                "-m",
                "elf_x86_64",
                scrt1,
                crti,
            ];

            args.extend(&["-o", &output_filename]);

            args.extend(&[
                "-L/lib64",
                "-L/usr/lib64",
                "-L/lib/x86_64-linux-gnu",
                "-zrelro",
                "--no-as-needed",
                "-lc",
                "-O1",
                crtn,
            ]);

            args.extend(objects.iter().map(|x| x.as_str()));

            args
        } else {
            unimplemented!()
        }
    };

    let mut linker = std::process::Command::new("ld");
    let proc = linker.args(args.iter()).spawn()?;
    let output = proc.wait_with_output()?;

    // TODO: propagate
    assert!(output.status.success());
    Ok(())
}

pub fn compile_binary(
    program: &Program,
    output_file: impl AsRef<Path>,
) -> Result<(), CodegenError> {
    let object_file = compile(program, &output_file)?;
    link_binary(&[object_file], output_file)?;
    Ok(())
}

pub fn link_shared_lib(
    objects: &[impl AsRef<Path>],
    output_filename: impl AsRef<Path>,
) -> std::io::Result<()> {
    let mut output_filename = output_filename.as_ref().to_path_buf();
    let objects: Vec<_> = objects
        .iter()
        .map(|x| x.as_ref().display().to_string())
        .collect();

    if output_filename.extension().is_none() {
        output_filename = output_filename.with_extension(get_platform_library_ext());
    }

    let output_filename = output_filename.to_string_lossy().to_string();

    let args: Vec<_> = {
        if cfg!(target_os = "macos") {
            let mut args = vec![
                "-demangle",
                "-no_deduplicate",
                "-dynamic",
                "-dylib",
                "-L/usr/local/lib",
                "-L/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk/usr/lib",
            ];

            args.extend(objects.iter().map(|x| x.as_str()));

            args.extend(&["-o", &output_filename, "-lSystem"]);

            args
        } else if cfg!(target_os = "linux") {
            let mut args = vec!["--hash-style=gnu", "--eh-frame-hdr", "-shared"];

            args.extend(&["-o", &output_filename]);

            args.extend(&["-L/lib/../lib64", "-L/usr/lib/../lib64", "-lc", "-O1"]);

            args.extend(objects.iter().map(|x| x.as_str()));

            args
        } else {
            unimplemented!()
        }
    };

    let mut linker = std::process::Command::new("ld");
    let proc = linker.args(args.iter()).spawn()?;
    proc.wait_with_output()?;
    Ok(())
}

pub fn get_platform_library_ext() -> &'static str {
    if cfg!(target_os = "macos") {
        "dylib"
    } else if cfg!(target_os = "windows") {
        "dll"
    } else {
        "so"
    }
}

pub fn compile_shared_lib(
    program: &Program,
    output_file: impl AsRef<Path>,
) -> Result<(), CodegenError> {
    let object_file = compile(program, &output_file)?;
    link_shared_lib(&[object_file], output_file)?;
    Ok(())
}

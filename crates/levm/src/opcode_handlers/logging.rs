// Logging Operations (5)
// Opcodes: LOG0 ... LOG4
use super::*;

// Implement empty op_log(n) method

impl VM {
    pub fn op_log(current_call_frame: &mut CallFrame, op: Opcode) -> Result<(), VMError> {
        if current_call_frame.is_static {
            return Err(VMError::OpcodeNotAllowedInStaticContext);
        }

        let number_of_topics = (op as u8) - (Opcode::LOG0 as u8);
        let offset = current_call_frame
            .stack
            .pop()?
            .try_into()
            .unwrap_or(usize::MAX);
        let size = current_call_frame
            .stack
            .pop()?
            .try_into()
            .unwrap_or(usize::MAX);
        let mut topics = Vec::new();
        for _ in 0..number_of_topics {
            let topic = current_call_frame.stack.pop()?.as_u32();
            topics.push(H32::from_slice(topic.to_be_bytes().as_ref()));
        }

        let data = current_call_frame.memory.load_range(offset, size);
        let log = Log {
            address: current_call_frame.msg_sender, // Should change the addr if we are on a Call/Create transaction (Call should be the contract we are calling, Create should be the original caller)
            topics,
            data: Bytes::from(data),
        };
        current_call_frame.logs.push(log);
        Ok(())
    }
}

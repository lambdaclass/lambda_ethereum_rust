use ethereum_rust_rlp::encode::RLPEncode;
use ethereum_rust_storage::{error::StoreError, Store};

use crate::rlpx::snap::{AccountRange, AccountStateSlim, GetAccountRange};

pub fn process_account_range_request(
    request: GetAccountRange,
    store: Store,
) -> Result<AccountRange, StoreError> {
    let mut accounts = vec![];
    // Fetch account range
    let mut iter = store.iter_accounts(request.root_hash);
    let mut start_found = false;
    let mut bytes_used = 0;
    while let Some((k, v)) = iter.next() {
        if k >= request.starting_hash {
            start_found = true;
        }
        if start_found {
            let acc = AccountStateSlim::from(v);
            bytes_used += bytes_per_entry(&acc);
            accounts.push((k, acc));
        }
        if k >= request.limit_hash {
            break;
        }
        if bytes_used >= request.response_bytes {
            break;
        }
    }
    let proof = store.get_account_range_proof(request.root_hash, request.starting_hash)?;

    Ok(AccountRange {
        id: request.id,
        accounts,
        proof,
    })
}

// TODO: write response bytes directly here so we dont need to encode twice
fn bytes_per_entry(state: &AccountStateSlim) -> u64 {
    state.encode_to_vec().len() as u64 + 32
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use ethereum_rust_core::{types::AccountState, BigEndianHash, H256};
    use ethereum_rust_rlp::{decode::RLPDecode, encode::RLPEncode};
    use ethereum_rust_storage::EngineType;

    use crate::rlpx::snap::AccountStateSlim;

    use super::*;

    // Hive `AccounRange` Tests
    // Requests & invariantes taken from https://github.com/ethereum/go-ethereum/blob/3e567b8b2901611f004b5a6070a9b6d286be128d/cmd/devp2p/internal/ethtest/snap.go#L69

    use lazy_static::lazy_static;

    lazy_static! {
        static ref HASH_MIN: H256 = H256::zero();
        static ref HASH_MAX: H256 =
            H256::from_str("0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",)
                .unwrap();
        static ref HASH_FIRST: H256 =
            H256::from_str("0x005e94bf632e80cde11add7d3447cd4ca93a5f2205d9874261484ae180718bd6")
                .unwrap();
        static ref HASH_SECOND: H256 =
            H256::from_str("0x00748bacab20da9ae19dd26a33bd10bbf825e28b3de84fc8fe1d15a21645067f")
                .unwrap();
        static ref HASH_FIRST_MINUS_500: H256 = H256::from_uint(&((*HASH_FIRST).into_uint() - 500));
        static ref HASH_FIRST_MINUS_450: H256 = H256::from_uint(&((*HASH_FIRST).into_uint() - 450));
        static ref HASH_FIRST_PLUS_ONE: H256 = H256::from_uint(&((*HASH_FIRST).into_uint() + 1));
    }

    #[test]
    fn hive_account_range_a() {
        let (store, root) = setup_initial_state();
        let request = GetAccountRange {
            id: 0,
            root_hash: root,
            starting_hash: *HASH_MIN,
            limit_hash: *HASH_MAX,
            response_bytes: 4000,
        };
        let res = process_account_range_request(request, store).unwrap();
        // Check test invariants
        assert_eq!(res.accounts.len(), 86);
        assert_eq!(res.accounts.first().unwrap().0, *HASH_FIRST);
        assert_eq!(
            res.accounts.last().unwrap().0,
            H256::from_str("0x445cb5c1278fdce2f9cbdb681bdd76c52f8e50e41dbd9e220242a69ba99ac099")
                .unwrap()
        );
        // Check proofs against geth values
    }

    #[test]
    fn hive_account_range_b() {
        let (store, root) = setup_initial_state();
        let request = GetAccountRange {
            id: 0,
            root_hash: root,
            starting_hash: *HASH_MIN,
            limit_hash: *HASH_MAX,
            response_bytes: 3000,
        };
        let res = process_account_range_request(request, store).unwrap();
        // Check test invariants
        assert_eq!(res.accounts.len(), 65);
        assert_eq!(res.accounts.first().unwrap().0, *HASH_FIRST);
        assert_eq!(
            res.accounts.last().unwrap().0,
            H256::from_str("0x2e6fe1362b3e388184fd7bf08e99e74170b26361624ffd1c5f646da7067b58b6")
                .unwrap()
        );
    }

    #[test]
    fn hive_account_range_c() {
        let (store, root) = setup_initial_state();
        let request = GetAccountRange {
            id: 0,
            root_hash: root,
            starting_hash: *HASH_MIN,
            limit_hash: *HASH_MAX,
            response_bytes: 2000,
        };
        let res = process_account_range_request(request, store).unwrap();
        // Check test invariants
        assert_eq!(res.accounts.len(), 44);
        assert_eq!(res.accounts.first().unwrap().0, *HASH_FIRST);
        assert_eq!(
            res.accounts.last().unwrap().0,
            H256::from_str("0x1c3f74249a4892081ba0634a819aec9ed25f34c7653f5719b9098487e65ab595")
                .unwrap()
        );
    }

    #[test]
    fn hive_account_range_d() {
        let (store, root) = setup_initial_state();
        let request = GetAccountRange {
            id: 0,
            root_hash: root,
            starting_hash: *HASH_MIN,
            limit_hash: *HASH_MAX,
            response_bytes: 1,
        };
        let res = process_account_range_request(request, store).unwrap();
        // Check test invariants
        assert_eq!(res.accounts.len(), 1);
        assert_eq!(res.accounts.first().unwrap().0, *HASH_FIRST);
        assert_eq!(res.accounts.last().unwrap().0, *HASH_FIRST);
    }

    #[test]
    fn hive_account_range_e() {
        let (store, root) = setup_initial_state();
        let request = GetAccountRange {
            id: 0,
            root_hash: root,
            starting_hash: *HASH_MIN,
            limit_hash: *HASH_MAX,
            response_bytes: 0,
        };
        let res = process_account_range_request(request, store).unwrap();
        // Check test invariants
        assert_eq!(res.accounts.len(), 1);
        assert_eq!(res.accounts.first().unwrap().0, *HASH_FIRST);
        assert_eq!(res.accounts.last().unwrap().0, *HASH_FIRST);
    }

    #[test]
    fn hive_account_range_f() {
        // In this test, we request a range where startingHash is before the first available
        // account key, and limitHash is after. The server should return the first and second
        // account of the state (because the second account is the 'next available').
        let (store, root) = setup_initial_state();
        let request = GetAccountRange {
            id: 0,
            root_hash: root,
            starting_hash: *HASH_FIRST_MINUS_500,
            limit_hash: *HASH_FIRST_PLUS_ONE,
            response_bytes: 4000,
        };
        let res = process_account_range_request(request, store).unwrap();
        // Check test invariants
        assert_eq!(res.accounts.len(), 2);
        assert_eq!(res.accounts.first().unwrap().0, *HASH_FIRST);
        assert_eq!(res.accounts.last().unwrap().0, *HASH_SECOND);
    }

    #[test]
    fn hive_account_range_g() {
        // Here we request range where both bounds are before the first available account key.
        // This should return the first account (even though it's out of bounds).
        let (store, root) = setup_initial_state();
        let request = GetAccountRange {
            id: 0,
            root_hash: root,
            starting_hash: *HASH_FIRST_MINUS_500,
            limit_hash: *HASH_FIRST_MINUS_450,
            response_bytes: 4000,
        };
        let res = process_account_range_request(request, store).unwrap();
        // Check test invariants
        assert_eq!(res.accounts.len(), 1);
        assert_eq!(res.accounts.first().unwrap().0, *HASH_FIRST);
        assert_eq!(res.accounts.last().unwrap().0, *HASH_FIRST);
    }

    #[test]
    fn hive_account_range_h() {
        // In this test, both startingHash and limitHash are zero.
        // The server should return the first available account.
        let (store, root) = setup_initial_state();
        let request = GetAccountRange {
            id: 0,
            root_hash: root,
            starting_hash: *HASH_MIN,
            limit_hash: *HASH_MIN,
            response_bytes: 4000,
        };
        let res = process_account_range_request(request, store).unwrap();
        // Check test invariants
        assert_eq!(res.accounts.len(), 1);
        assert_eq!(res.accounts.first().unwrap().0, *HASH_FIRST);
        assert_eq!(res.accounts.last().unwrap().0, *HASH_FIRST);
    }

    // Initial state setup for hive snap tests

    fn setup_initial_state() -> (Store, H256) {
        // We cannot process the old blocks that hive uses for the devp2p snap tests
        // So I took the state from a geth execution to run them locally

        let accounts: Vec<(&str, Vec<u8>)> = vec![
            (
                "0x005e94bf632e80cde11add7d3447cd4ca93a5f2205d9874261484ae180718bd6",
                vec![
                    228_u8, 1, 128, 160, 223, 151, 249, 75, 196, 116, 113, 135, 6, 6, 246, 38, 251,
                    122, 11, 66, 238, 210, 212, 95, 204, 132, 220, 18, 0, 206, 98, 247, 131, 29,
                    169, 144, 128,
                ],
            ),
            (
                "0x00748bacab20da9ae19dd26a33bd10bbf825e28b3de84fc8fe1d15a21645067f",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x00aa781aff39a8284ef43790e3a511b2caa50803613c5096bc782e8de08fa4c5",
                vec![
                    211, 128, 143, 192, 151, 206, 123, 201, 7, 21, 179, 75, 159, 16, 0, 0, 0, 0,
                    128, 128,
                ],
            ),
            (
                "0x016d92531f4754834b0502de5b0342ceff21cde5bef386a83d2292f4445782c2",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x02547b56492bfe767f3d18be2aab96441c449cd945770ef7ef8555acc505b2e4",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x025f478d53bf78add6fa3708d9e061d59bfe14b21329b2a4cf1156d4f81b3d2d",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0x0267c643f67b47cac9efacf6fcf0e4f4e1b273a727ded155db60eb9907939eb6",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0x0304d8eaccf0b942c468074250cbcb625ec5c4688b6b5d17d2a9bdd8dd565d5a",
                vec![
                    228, 1, 128, 160, 224, 12, 73, 166, 88, 73, 208, 92, 191, 39, 164, 215, 120,
                    138, 104, 188, 123, 96, 19, 174, 51, 65, 29, 64, 188, 137, 40, 47, 192, 100,
                    243, 61, 128,
                ],
            ),
            (
                "0x0463e52cda557221b0b66bd7285b043071df4c2ab146260f4e010970f3a0cccf",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0x04d9aa4f67f8b24d70a0ffd757e82456d9184113106b7d9e8eb6c3e8a8df27ee",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0x053df2c3b574026812b154a99b13b626220af85cd01bb1693b1d42591054bce6",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0x0579e46a5ed8a88504ac7d579b12eb346fbe4fd7e281bdd226b891f8abed4789",
                vec![
                    228, 1, 128, 160, 61, 14, 43, 165, 55, 243, 89, 65, 6, 135, 9, 69, 15, 37, 254,
                    228, 90, 175, 77, 198, 174, 46, 210, 42, 209, 46, 7, 67, 172, 124, 84, 167,
                    128,
                ],
            ),
            (
                "0x05f6de281d8c2b5d98e8e01cd529bd76416b248caf11e0552047c5f1d516aab6",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x07b49045c401bcc408f983d91a199c908cdf0d646049b5b83629a70b0117e295",
                vec![
                    228, 1, 128, 160, 134, 154, 203, 146, 159, 89, 28, 84, 203, 133, 132, 42, 81,
                    242, 150, 99, 94, 125, 137, 87, 152, 197, 71, 162, 147, 175, 228, 62, 123, 247,
                    244, 23, 128,
                ],
            ),
            (
                "0x0993fd5b750fe4414f93c7880b89744abb96f7af1171ed5f47026bdf01df1874",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x099d5081762b8b265e8ba4cd8e43f08be4715d903a0b1d96b3d9c4e811cbfb33",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x09d6e6745d272389182a510994e2b54d14b731fac96b9c9ef434bc1924315371",
                vec![196, 128, 128, 128, 128],
            ),
            (
                "0x0a93a7231976ad485379a3b66c2d8983ba0b2ca87abaf0ca44836b2a06a2b102",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x0b564e4a0203cbcec8301709a7449e2e7371910778df64c89f48507390f2d129",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0x0cd2a7c53c76f228ed3aa7a29644b1915fde9ec22e0433808bf5467d914e7c7a",
                vec![
                    228, 1, 128, 160, 7, 84, 3, 90, 164, 7, 51, 129, 162, 17, 52, 43, 80, 125, 232,
                    231, 117, 201, 124, 150, 16, 150, 230, 226, 39, 93, 240, 191, 203, 179, 160,
                    28, 128,
                ],
            ),
            (
                "0x0e0e4646090b881949ec9991e48dec768ccd1980896aefd0d51fd56fd5689790",
                vec![
                    228, 1, 128, 160, 96, 252, 105, 16, 13, 142, 99, 38, 103, 200, 11, 148, 212,
                    52, 0, 136, 35, 237, 117, 65, 107, 113, 203, 209, 18, 180, 208, 176, 47, 86,
                    48, 39, 128,
                ],
            ),
            (
                "0x0e27113c09de0a0cb0ff268c677aba17d39a3190fe15aec0ff7f54184955cba4",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0x0e57ffa6cc6cbd96c1400150417dd9b30d958c58f63c36230a90a02b076f78b5",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x0f30822f90f33f1d1ba6d1521a00935630d2c81ab12fa03d4a0f4915033134f3",
                vec![
                    228, 1, 128, 160, 128, 120, 243, 37, 157, 129, 153, 183, 202, 57, 213, 30, 53,
                    213, 181, 141, 113, 255, 20, 134, 6, 115, 16, 96, 56, 109, 50, 60, 93, 25, 24,
                    44, 128,
                ],
            ),
            (
                "0x1017b10a7cc3732d729fe1f71ced25e5b7bc73dc62ca61309a8c7e5ac0af2f72",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0x1098f06082dc467088ecedb143f9464ebb02f19dc10bd7491b03ba68d751ce45",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0x11eb0304c1baa92e67239f6947cb93e485a7db05e2b477e1167a8960458fa8cc",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0x12be3bf1f9b1dab5f908ca964115bee3bcff5371f84ede45bc60591b21117c51",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0x12c1bb3dddf0f06f62d70ed5b7f7db7d89b591b3f23a838062631c4809c37196",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x12e394ad62e51261b4b95c431496e46a39055d7ada7dbf243f938b6d79054630",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0x13cfc46f6bdb7a1c30448d41880d061c3b8d36c55a29f1c0c8d95a8e882b8c25",
                vec![
                    228, 1, 128, 160, 148, 79, 9, 90, 251, 209, 56, 62, 93, 15, 145, 239, 2, 137,
                    93, 57, 143, 79, 118, 253, 182, 216, 106, 223, 71, 101, 242, 91, 220, 48, 79,
                    95, 128,
                ],
            ),
            (
                "0x15293aec87177f6c88f58bc51274ba75f1331f5cb94f0c973b1deab8b3524dfe",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x170c927130fe8f1db3ae682c22b57f33f54eb987a7902ec251fe5dba358a2b25",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x17350c7adae7f08d7bbb8befcc97234462831638443cd6dfea186cbf5a08b7c7",
                vec![
                    228, 1, 128, 160, 76, 231, 156, 217, 100, 86, 80, 240, 160, 14, 255, 168, 111,
                    111, 234, 115, 60, 236, 234, 158, 162, 105, 100, 130, 143, 242, 92, 240, 87,
                    123, 201, 116, 128,
                ],
            ),
            (
                "0x174f1a19ff1d9ef72d0988653f31074cb59e2cf37cd9d2992c7b0dd3d77d84f9",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x17984cc4b4aac0492699d37662b53ec2acf8cbe540c968b817061e4ed27026d0",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x181abdd5e212171007e085fdc284a84d42d5bfc160960d881ccb6a10005ff089",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0x188111c233bf6516bb9da8b5c4c31809a42e8604cd0158d933435cfd8e06e413",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0x18f4256a59e1b2e01e96ac465e1d14a45d789ce49728f42082289fc25cf32b8d",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x1960414a11f8896c7fc4243aba7ed8179b0bc6979b7c25da7557b17f5dee7bf7",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0x1a28912018f78f7e754df6b9fcec33bea25e5a232224db622e0c3343cf079eff",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0x1bf7626cec5330a127e439e68e6ee1a1537e73b2de1aa6d6f7e06bc0f1e9d763",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x1c248f110218eaae2feb51bc82e9dcc2844bf93b88172c52afcb86383d262323",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0x1c3f74249a4892081ba0634a819aec9ed25f34c7653f5719b9098487e65ab595",
                vec![
                    228, 1, 128, 160, 175, 134, 126, 108, 186, 232, 16, 202, 169, 36, 184, 182,
                    172, 61, 140, 8, 145, 131, 20, 145, 166, 144, 109, 208, 190, 122, 211, 36, 220,
                    209, 83, 61, 128,
                ],
            ),
            (
                "0x1d38ada74301c31f3fd7d92dd5ce52dc37ae633e82ac29c4ef18dfc141298e26",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0x1d6ee979097e29141ad6b97ae19bb592420652b7000003c55eb52d5225c3307d",
                vec![
                    228, 1, 128, 160, 247, 53, 145, 231, 145, 175, 76, 124, 95, 160, 57, 195, 61,
                    217, 209, 105, 202, 177, 75, 29, 155, 12, 167, 139, 204, 78, 116, 13, 85, 59,
                    26, 207, 128,
                ],
            ),
            (
                "0x1dff76635b74ddba16bba3054cc568eed2571ea6becaabd0592b980463f157e2",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0x1ee7e0292fba90d9733f619f976a2655c484adb30135ef0c5153b5a2f32169df",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0x209b102e507b8dfc6acfe2cf55f4133b9209357af679a6d507e6ee87112bfe10",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0x210ce6d692a21d75de3764b6c0356c63a51550ebec2c01f56c154c24b1cf8888",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0x2116ab29b4cb8547af547fe472b7ce30713f234ed49cb1801ea6d3cf9c796d57",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0x2290ea88cc63f09ab5e8c989a67e2e06613311801e39c84aae3badd8bb38409c",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0x2369a492b6cddcc0218617a060b40df0e7dda26abe48ba4e4108c532d3f2b84f",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0x2374954008440ca3d17b1472d34cc52a6493a94fb490d5fb427184d7d5fd1cbf",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0x23ddaac09188c12e5d88009afa4a34041175c5531f45be53f1560a1cbfec4e8a",
                vec![
                    228, 1, 128, 160, 71, 250, 72, 226, 93, 54, 105, 169, 187, 25, 12, 89, 147,
                    143, 75, 228, 157, 226, 208, 131, 105, 110, 185, 57, 195, 180, 7, 46, 198, 126,
                    67, 177, 128,
                ],
            ),
            (
                "0x246cc8a2b79a30ec71390d829d0cb37cce1b953e89cb14deae4945526714a71c",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x255ec86eac03ba59f6dfcaa02128adbb22c561ae0c49e9e62e4fff363750626e",
                vec![
                    228, 1, 128, 160, 102, 235, 22, 7, 27, 163, 121, 191, 12, 99, 47, 203, 82, 249,
                    23, 90, 101, 107, 239, 98, 173, 240, 190, 245, 52, 154, 127, 90, 106, 173, 93,
                    136, 128,
                ],
            ),
            (
                "0x26ce7d83dfb0ab0e7f15c42aeb9e8c0c5dba538b07c8e64b35fb64a37267dd96",
                vec![
                    228, 1, 128, 160, 36, 52, 191, 198, 67, 236, 54, 65, 22, 205, 113, 81, 154, 57,
                    118, 98, 178, 12, 82, 209, 173, 207, 240, 184, 48, 232, 10, 115, 142, 25, 243,
                    14, 128,
                ],
            ),
            (
                "0x2705244734f69af78e16c74784e1dc921cb8b6a98fe76f577cc441c831e973bf",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0x28f25652ec67d8df6a2e33730e5d0983443e3f759792a0128c06756e8eb6c37f",
                vec![
                    211, 128, 143, 192, 151, 206, 123, 201, 7, 21, 179, 75, 159, 16, 0, 0, 0, 0,
                    128, 128,
                ],
            ),
            (
                "0x2a248c1755e977920284c8054fceeb20530dc07cd8bbe876f3ce02000818cc3a",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0x2a39afbe88f572c23c90da2d059af3de125f1da5c3753c530dc5619a4857119f",
                vec![
                    228, 1, 128, 160, 130, 137, 181, 88, 134, 95, 44, 161, 245, 76, 152, 181, 255,
                    93, 249, 95, 7, 194, 78, 198, 5, 226, 71, 181, 140, 119, 152, 96, 93, 205, 121,
                    79, 128,
                ],
            ),
            (
                "0x2b8d12301a8af18405b3c826b6edcc60e8e034810f00716ca48bebb84c4ce7ab",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0x2baa718b760c0cbd0ec40a3c6df7f2948b40ba096e6e4b116b636f0cca023bde",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x2e6fe1362b3e388184fd7bf08e99e74170b26361624ffd1c5f646da7067b58b6",
                vec![
                    228, 128, 128, 128, 160, 142, 3, 136, 236, 246, 76, 250, 118, 179, 166, 175,
                    21, 159, 119, 69, 21, 25, 167, 249, 187, 134, 46, 76, 206, 36, 23, 92, 121, 31,
                    220, 176, 223,
                ],
            ),
            (
                "0x2fe5767f605b7b821675b223a22e4e5055154f75e7f3041fdffaa02e4787fab8",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x303f57a0355c50bf1a0e1cf0fa8f9bdbc8d443b70f2ad93ac1c6b9c1d1fe29a2",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0x30ce5b7591126d5464dfb4fc576a970b1368475ce097e244132b06d8cc8ccffe",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x315ccc15883d06b4e743f8252c999bf1ee994583ff6114d89c0f3ddee828302b",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0x3197690074092fe51694bdb96aaab9ae94dac87f129785e498ab171a363d3b40",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x34a715e08b77afd68cde30b62e222542f3db90758370400c94d0563959a1d1a0",
                vec![
                    228, 1, 128, 160, 79, 68, 99, 41, 181, 238, 61, 19, 212, 246, 181, 229, 242,
                    16, 221, 194, 217, 15, 237, 186, 56, 75, 149, 14, 54, 161, 209, 154, 249, 92,
                    92, 177, 128,
                ],
            ),
            (
                "0x37310559ceaade42e45b3e3f05925aadca9e60aeeb9dd60d824875d9e9e71e26",
                vec![
                    228, 1, 128, 160, 114, 200, 146, 33, 218, 237, 204, 221, 63, 187, 166, 108, 27,
                    8, 27, 54, 52, 206, 137, 213, 160, 105, 190, 151, 255, 120, 50, 119, 143, 123,
                    2, 58, 128,
                ],
            ),
            (
                "0x37d65eaa92c6bc4c13a5ec45527f0c18ea8932588728769ec7aecfe6d9f32e42",
                vec![
                    248, 68, 128, 42, 160, 172, 49, 98, 168, 185, 219, 180, 49, 139, 132, 33, 159,
                    49, 64, 231, 169, 236, 53, 18, 98, 52, 18, 2, 151, 221, 225, 15, 81, 178, 95,
                    106, 38, 160, 245, 122, 205, 64, 37, 152, 114, 96, 109, 118, 25, 126, 240, 82,
                    243, 211, 85, 136, 218, 223, 145, 158, 225, 240, 227, 203, 155, 98, 211, 244,
                    176, 44,
                ],
            ),
            (
                "0x37ddfcbcb4b2498578f90e0fcfef9965dcde4d4dfabe2f2836d2257faa169947",
                vec![
                    228, 1, 128, 160, 82, 214, 210, 145, 58, 228, 75, 202, 17, 181, 161, 22, 2, 29,
                    185, 124, 145, 161, 62, 56, 94, 212, 139, 160, 102, 40, 231, 66, 1, 35, 29,
                    186, 128,
                ],
            ),
            (
                "0x37e51740ad994839549a56ef8606d71ace79adc5f55c988958d1c450eea5ac2d",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0x38152bce526b7e1c2bedfc9d297250fcead02818be7806638564377af145103b",
                vec![
                    228, 1, 128, 160, 108, 0, 224, 145, 218, 227, 212, 34, 111, 172, 214, 190, 128,
                    44, 134, 93, 93, 176, 245, 36, 117, 77, 34, 102, 100, 6, 19, 139, 84, 250, 176,
                    230, 128,
                ],
            ),
            (
                "0x3848b7da914222540b71e398081d04e3849d2ee0d328168a3cc173a1cd4e783b",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0x389093badcaa24c3a8cbb4461f262fba44c4f178a162664087924e85f3d55710",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0x3897cb9b6f68765022f3c74f84a9f2833132858f661f4bc91ccd7a98f4e5b1ee",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0x395b92f75f8e06b5378a84ba03379f025d785d8b626b2b6a1c84b718244b9a91",
                vec![
                    228, 1, 128, 160, 84, 70, 184, 24, 244, 198, 105, 102, 156, 211, 49, 71, 38,
                    255, 19, 76, 241, 140, 88, 169, 165, 54, 223, 19, 199, 0, 97, 7, 5, 168, 183,
                    200, 128,
                ],
            ),
            (
                "0x3be526914a7d688e00adca06a0c47c580cb7aa934115ca26006a1ed5455dd2ce",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x3e57e37bc3f588c244ffe4da1f48a360fa540b77c92f0c76919ec4ee22b63599",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x415ded122ff7b6fe5862f5c443ea0375e372862b9001c5fe527d276a3a420280",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0x419809ad1512ed1ab3fb570f98ceb2f1d1b5dea39578583cd2b03e9378bbe418",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0x4363d332a0d4df8582a84932729892387c623fe1ec42e2cfcbe85c183ed98e0e",
                vec![
                    213, 130, 1, 146, 143, 192, 151, 206, 123, 201, 7, 21, 179, 73, 233, 122, 138,
                    101, 46, 31, 128, 128,
                ],
            ),
            (
                "0x445cb5c1278fdce2f9cbdb681bdd76c52f8e50e41dbd9e220242a69ba99ac099",
                vec![
                    228, 1, 1, 160, 190, 61, 117, 161, 114, 155, 225, 87, 231, 156, 59, 119, 240,
                    2, 6, 219, 77, 84, 227, 234, 20, 55, 90, 1, 84, 81, 200, 142, 192, 103, 199,
                    144, 128,
                ],
            ),
            (
                "0x4615e5f5df5b25349a00ad313c6cd0436b6c08ee5826e33a018661997f85ebaa",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0x465311df0bf146d43750ed7d11b0451b5f6d5bfc69b8a216ef2f1c79c93cd848",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x47450e5beefbd5e3a3f80cbbac474bb3db98d5e609aa8d15485c3f0d733dea3a",
                vec![
                    228, 1, 128, 160, 84, 66, 224, 39, 157, 63, 17, 73, 222, 76, 232, 217, 226,
                    211, 240, 29, 24, 84, 117, 80, 56, 172, 26, 15, 174, 92, 72, 116, 155, 247, 31,
                    32, 128,
                ],
            ),
            (
                "0x482814ea8f103c39dcf6ba7e75df37145bde813964d82e81e5d7e3747b95303d",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x4845aac9f26fcd628b39b83d1ccb5c554450b9666b66f83aa93a1523f4db0ab6",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0x48e291f8a256ab15da8401c8cae555d5417a992dff3848926fa5b71655740059",
                vec![
                    228, 1, 128, 160, 162, 231, 8, 75, 169, 206, 193, 121, 81, 156, 126, 137, 80,
                    198, 106, 211, 203, 168, 88, 106, 96, 207, 249, 244, 214, 12, 24, 141, 214, 33,
                    82, 42, 128,
                ],
            ),
            (
                "0x4973f6aa8cf5b1190fc95379aa01cff99570ee6b670725880217237fb49e4b24",
                vec![
                    228, 1, 128, 160, 174, 46, 127, 28, 147, 60, 108, 168, 76, 232, 190, 129, 30,
                    244, 17, 222, 231, 115, 251, 105, 80, 128, 86, 215, 36, 72, 4, 142, 161, 219,
                    92, 71, 128,
                ],
            ),
            (
                "0x4b238e08b80378d0815e109f350a08e5d41ec4094df2cfce7bc8b9e3115bda70",
                vec![
                    228, 1, 128, 160, 17, 245, 211, 153, 202, 143, 183, 169, 175, 90, 212, 129,
                    190, 96, 207, 97, 212, 84, 147, 205, 32, 32, 108, 157, 10, 35, 124, 231, 215,
                    87, 30, 95, 128,
                ],
            ),
            (
                "0x4b9f335ce0bdffdd77fdb9830961c5bc7090ae94703d0392d3f0ff10e6a4fbab",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0x4bd8ef9873a5e85d4805dbcb0dbf6810e558ea175167549ef80545a9cafbb0e1",
                vec![
                    228, 1, 128, 160, 161, 73, 19, 213, 72, 172, 29, 63, 153, 98, 162, 26, 86, 159,
                    229, 47, 20, 54, 182, 210, 245, 234, 78, 54, 222, 19, 234, 133, 94, 222, 84,
                    224, 128,
                ],
            ),
            (
                "0x4c2765139cace1d217e238cc7ccfbb751ef200e0eae7ec244e77f37e92dfaee5",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0x4c310e1f5d2f2e03562c4a5c473ae044b9ee19411f07097ced41e85bd99c3364",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x4ccd31891378d2025ef58980481608f11f5b35a988e877652e7cbb0a6127287c",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0x4ceaf2371fcfb54a4d8bc1c804d90b06b3c32c9f17112b57c29b30a25cf8ca12",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x4d67d989fdb264fa4b2524d306f7b3f70ddce0b723411581d1740407da325462",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x4d79fea6c7fef10cb0b5a8b3d85b66836a131bec0b04d891864e6fdb9794af75",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0x4e0ab2902f57bf2a250c0f87f088acc325d55f2320f2e33abd8e50ba273c9244",
                vec![
                    228, 1, 128, 160, 193, 104, 96, 69, 40, 138, 89, 82, 173, 87, 222, 14, 151, 27,
                    210, 80, 7, 114, 60, 159, 116, 159, 73, 243, 145, 231, 21, 194, 123, 245, 38,
                    200, 128,
                ],
            ),
            (
                "0x4e258aa445a0e2a8704cbc57bbe32b859a502cd6f99190162236300fabd86c4a",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0x4e5bab4ebd077c3bbd8239995455989ea2e95427ddeed47d0618d9773332bb05",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x4f458f480644b18c0e8207f405b82da7f75c7b3b5a34fe6771a0ecf644677f33",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0x4fbc5fc8df4f0a578c3be3549f1cb3ef135cbcdf75f620c7a1d412462e9b3b94",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0x4fd7c8d583447b937576211163a542d945ac8c0a6e22d0c42ac54e2cbaff9281",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x50d83ef5194d06752cd5594b57e809b135f24eedd124a51137feaaf049bc2efd",
                vec![
                    228, 1, 128, 160, 81, 184, 41, 240, 242, 195, 222, 156, 251, 217, 78, 71, 130,
                    138, 137, 148, 12, 50, 154, 73, 205, 89, 84, 12, 163, 198, 215, 81, 168, 210,
                    20, 214, 128,
                ],
            ),
            (
                "0x5162f18d40405c59ef279ad71d87fbec2bbfedc57139d56986fbf47daf8bcbf2",
                vec![
                    211, 128, 143, 192, 151, 206, 123, 201, 7, 21, 179, 75, 159, 16, 0, 0, 0, 0,
                    128, 128,
                ],
            ),
            (
                "0x517bd5fbe28e4368b0b9fcba13d5e81fb51babdf4ed63bd83885235ee67a8fa0",
                vec![
                    228, 1, 128, 160, 116, 237, 120, 235, 22, 1, 109, 127, 243, 161, 115, 171, 27,
                    188, 238, 157, 170, 142, 53, 138, 157, 108, 155, 229, 232, 75, 166, 244, 163,
                    76, 249, 106, 128,
                ],
            ),
            (
                "0x519abb269c3c5710f1979ca84192e020ba5c838bdd267b2d07436a187f171232",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0x5264e880ecf7b80afda6cc2a151bac470601ff8e376af91aaf913a36a30c4009",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x52d034ca6ebd21c7ba62a2ad3b6359aa4a1cdc88bdaa64bb2271d898777293ab",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0x5380c7b7ae81a58eb98d9c78de4a1fd7fd9535fc953ed2be602daaa41767312a",
                vec![
                    205, 128, 137, 12, 167, 152, 153, 113, 244, 250, 99, 97, 128, 128,
                ],
            ),
            (
                "0x54c12444ede3e2567dd7f4d9a06d4db8c6ab800d5b3863f8ff22a0db6d09bf24",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x55cab9586acb40e66f66147ff3a059cfcbbad785dddd5c0cc31cb43edf98a5d5",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0x55d0609468d8d4147a942e88cfc5f667daff850788d821889fbb03298924767c",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x5602444769b5fd1ddfca48e3c38f2ecad326fe2433f22b90f6566a38496bd426",
                vec![
                    211, 128, 143, 192, 151, 206, 123, 201, 7, 21, 179, 75, 159, 16, 0, 0, 0, 0,
                    128, 128,
                ],
            ),
            (
                "0x5677600b2af87d21fdab2ac8ed39bd1be2f790c04600de0400c1989040d9879c",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x570210539713235b442bbbad50c58bee81b70efd2dad78f99e41a6c462faeb43",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0x580aa878e2f92d113a12c0a3ce3c21972b03dbe80786858d49a72097e2c491a3",
                vec![
                    228, 1, 128, 160, 71, 27, 248, 152, 138, 208, 215, 96, 45, 107, 213, 73, 60, 8,
                    115, 48, 150, 193, 22, 172, 120, 139, 118, 242, 42, 104, 43, 196, 85, 142, 58,
                    167, 128,
                ],
            ),
            (
                "0x58e416a0dd96454bd2b1fe3138c3642f5dee52e011305c5c3416d97bc8ba5cf0",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0x59312f89c13e9e24c1cb8b103aa39a9b2800348d97a92c2c9e2a78fa02b70025",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0x5a356862c79afffd6a01af752d950e11490146e4d86dfb8ab1531e9aef4945a1",
                vec![
                    228, 1, 128, 160, 58, 41, 133, 198, 173, 166, 126, 86, 4, 185, 159, 162, 252,
                    26, 48, 42, 189, 13, 194, 65, 238, 127, 20, 196, 40, 250, 103, 212, 118, 134,
                    139, 182, 128,
                ],
            ),
            (
                "0x5a4a3feecfc77b402e938e28df0c4cbb874771cb3c5a92524f303cffb82a2862",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x5aa3b4a2ebdd402721c3953b724f4fe90900250bb4ef89ce417ec440da318cd6",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0x5b90bb05df9514b2d8e3a8feb3d6c8c22526b02398f289b42111426edc4fe6cf",
                vec![
                    228, 1, 128, 160, 40, 122, 204, 120, 105, 66, 31, 185, 244, 154, 53, 73, 185,
                    2, 251, 1, 183, 172, 204, 3, 34, 67, 189, 126, 26, 204, 216, 150, 93, 149, 217,
                    21, 128,
                ],
            ),
            (
                "0x5c1d92594d6377fe6423257781b382f94dffcde4fadbf571aa328f6eb18f8fcd",
                vec![
                    211, 128, 143, 192, 151, 206, 123, 201, 7, 21, 179, 75, 159, 16, 0, 0, 0, 0,
                    128, 128,
                ],
            ),
            (
                "0x5c20f6ee05edbb60beeab752d87412b2f6e12c8feefa2079e6bd989f814ed4da",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0x5d97d758e8800d37b6d452a1b1812d0afedba11f3411a17a8d51ee13a38d73f0",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0x5e88e876a3af177e6daafe173b67f186a53f1771a663747f26b278c5acb4c219",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0x5ec55391e89ac4c3cf9e61801cd13609e8757ab6ed08687237b789f666ea781b",
                vec![
                    228, 1, 128, 160, 199, 191, 43, 52, 41, 64, 101, 175, 185, 162, 193, 95, 144,
                    108, 186, 31, 122, 26, 159, 13, 163, 78, 169, 196, 102, 3, 181, 44, 174, 144,
                    40, 236, 128,
                ],
            ),
            (
                "0x5fc13d7452287b5a8e3c3be9e4f9057b5c2dd82aeaff4ed892c96fc944ec31e7",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x5fcd9b6fce3394ad1d44733056b3e5f6306240974a16f9de8e96ebdd14ae06b1",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0x600a7a5f41a67f6f759dcb664198f1c5d9b657fb51a870ce9e234e686dff008e",
                vec![
                    228, 1, 128, 160, 158, 218, 142, 182, 202, 3, 215, 196, 175, 228, 114, 121,
                    172, 201, 10, 69, 209, 178, 202, 106, 17, 175, 217, 82, 6, 248, 134, 141, 32,
                    82, 13, 6, 128,
                ],
            ),
            (
                "0x60535eeb3ffb721c1688b879368c61a54e13f8881bdef6bd4a17b8b92e050e06",
                vec![
                    228, 1, 128, 160, 251, 121, 2, 30, 127, 165, 75, 155, 210, 223, 100, 246, 219,
                    87, 137, 125, 82, 174, 133, 247, 193, 149, 175, 81, 141, 228, 130, 0, 161, 50,
                    94, 44, 128,
                ],
            ),
            (
                "0x606059a65065e5f41347f38754e6ddb99b2d709fbff259343d399a4f9832b48f",
                vec![
                    228, 1, 128, 160, 191, 186, 27, 194, 172, 66, 101, 95, 90, 151, 69, 11, 230,
                    43, 148, 48, 130, 34, 50, 241, 206, 73, 152, 234, 245, 35, 155, 12, 36, 59, 43,
                    132, 128,
                ],
            ),
            (
                "0x61088707d2910974000e63c2d1a376f4480ba19dde19c4e6a757aeb3d62d5439",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0x6188c4510d25576535a642b15b1dbdb8922fe572b099f504390f923c19799777",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x6225e8f52719d564e8217b5f5260b1d1aac2bcb959e54bc60c5f479116c321b8",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0x625e5c85d5f4b6385574b572709d0f704b097527a251b7c658c0c4441aef2af6",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0x64bfba8a4688bdee41c4b998e101567b8b56fea53d30ab85393f2d5b70c5da90",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0x64d0de66ea29cbcf7f237dae1c5f883fa6ff0ba52b90f696bb0348224dbc82ce",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0x65cf42efacdee07ed87a1c2de0752a4e3b959f33f9f9f8c77424ba759e01fcf2",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0x65e6b6521e4f1f97e80710581f42063392c9b33e0aeea4081a102a32238992ea",
                vec![
                    228, 1, 128, 160, 17, 212, 238, 199, 223, 82, 205, 84, 231, 70, 144, 164, 135,
                    136, 78, 86, 55, 25, 118, 194, 184, 196, 159, 252, 76, 143, 52, 131, 17, 102,
                    191, 78, 128,
                ],
            ),
            (
                "0x662d147a16d7c23a2ba6d3940133e65044a90985e26207501bfca9ae47a2468c",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x6641e3ed1f264cf275b53bb7012dabecf4c1fca700e3db989e314c24cc167074",
                vec![
                    228, 1, 128, 160, 15, 216, 233, 155, 27, 74, 180, 235, 140, 108, 34, 24, 34,
                    26, 230, 151, 140, 198, 116, 51, 52, 30, 216, 161, 173, 97, 133, 211, 79, 168,
                    44, 97, 128,
                ],
            ),
            (
                "0x67cc0bf5341efbb7c8e1bdbf83d812b72170e6edec0263eeebdea6f107bbef0d",
                vec![
                    228, 1, 128, 160, 162, 14, 106, 33, 36, 74, 248, 255, 204, 213, 68, 34, 151,
                    173, 155, 122, 118, 172, 114, 215, 216, 172, 158, 22, 241, 47, 204, 80, 233,
                    11, 115, 78, 128,
                ],
            ),
            (
                "0x68fc814efedf52ac8032da358ddcb61eab4138cb56b536884b86e229c995689c",
                vec![
                    228, 1, 128, 160, 109, 43, 138, 7, 76, 120, 160, 229, 168, 9, 93, 122, 1, 13,
                    73, 97, 198, 57, 197, 65, 207, 86, 251, 183, 4, 148, 128, 204, 143, 25, 151,
                    101, 128,
                ],
            ),
            (
                "0x6a2c8498657ae4f0f7b1a02492c554f7f8a077e454550727890188f7423ba014",
                vec![
                    228, 1, 128, 160, 86, 34, 128, 27, 16, 17, 222, 132, 3, 228, 67, 8, 187, 248,
                    154, 88, 9, 183, 173, 101, 134, 38, 140, 215, 33, 100, 82, 53, 135, 249, 176,
                    228, 128,
                ],
            ),
            (
                "0x6a5e43139d88da6cfba857e458ae0b5359c3fde36e362b6e5f782a90ce351f14",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0x6ad3ba011e031431dc057c808b85346d58001b85b32a4b5c90ccccea0f82e170",
                vec![
                    228, 1, 128, 160, 20, 249, 244, 185, 68, 92, 117, 71, 213, 164, 103, 26, 56,
                    176, 177, 43, 188, 14, 113, 152, 195, 178, 147, 75, 130, 182, 149, 200, 99, 13,
                    73, 114, 128,
                ],
            ),
            (
                "0x6bd9fb206b22c76b4f9630248940855b842c684db89adff0eb9371846ea625a9",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0x6c05d8abc81143ce7c7568c98aadfe6561635c049c07b2b4bce3019cef328cb9",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x6c37093a34016ae687da7aabb18e42009b71edff70a94733c904aea51a4853c1",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0x6d1da4cf1127d654ed731a93105f481b315ecfc2f62b1ccb5f6d2717d6a40f9b",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0x6d4162ce16817e46fa2ddc5e70cee790b80abc3d6f7778cfbaed327c5d2af36c",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0x6dbe5551f50400859d14228606bf221beff07238bfa3866454304abb572f9512",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x6dc09fdec00aa9a30dd8db984406a33e3ca15e35222a74773071207a5e56d2c2",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0x6f358b4e903d31fdd5c05cddaa174296bb30b6b2f72f1ff6410e6c1069198989",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x7026c939a9158beedff127a64f07a98b328c3d1770690437afdb21c34560fc57",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x70aae390a762a4347a4d167a2431874554edf1d77579213e55fea3ec39a1257c",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0x71dee9adfef0940a36336903bd6830964865180b98c0506f9bf7ba8f2740fbf9",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0x720f25b62fc39426f70eb219c9dd481c1621821c8c0fa5367a1df6e59e3edf59",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0x728325587fa336e318b54298e1701d246c4f90d6094eb95635d8a47f080f4603",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x729953a43ed6c913df957172680a17e5735143ad767bda8f58ac84ec62fbec5e",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0x72d91596112f9d7e61d09ffa7575f3587ad9636172ae09641882761cc369ecc0",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0x72e962dfe7e2828809f5906996dedeba50950140555b193fceb94f12fd6f0a22",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0x734ee4981754a3f1403c4e8887d35addfb31717d93de3e00ede78368c230861e",
                vec![
                    228, 1, 128, 160, 44, 242, 146, 193, 227, 130, 189, 208, 231, 46, 18, 103, 1,
                    215, 176, 36, 132, 230, 226, 114, 244, 192, 216, 20, 245, 166, 250, 226, 51,
                    252, 121, 53, 128,
                ],
            ),
            (
                "0x73cd1b7cd355f3f77c570a01100a616757408bb7abb78fe9ee1262b99688fcc4",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0x74614a0c4ba7d7c70b162dad186b6cc77984ab4070534ad9757e04a5b776dcc8",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0x7583557e4e3918c95965fb610dc1424976c0eee606151b6dfc13640e69e5cb15",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0x75d231f57a1a9751f58769d5691f4807ab31ac0e802b1a1f6bfc77f5dff0adbf",
                vec![
                    228, 1, 128, 160, 205, 49, 237, 93, 93, 167, 153, 144, 175, 237, 13, 153, 60,
                    183, 37, 196, 227, 77, 217, 117, 68, 176, 52, 102, 237, 52, 33, 46, 66, 194,
                    141, 104, 128,
                ],
            ),
            (
                "0x78948842ff476b87544c189ce744d4d924ffd0907107a0dbaa4b71d0514f2225",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0x792cc9f20a61c16646d5b6136693e7789549adb7d8e35503d0004130ea6528b0",
                vec![
                    228, 1, 128, 160, 154, 74, 51, 249, 120, 216, 78, 10, 206, 179, 172, 54, 112,
                    194, 226, 223, 108, 138, 226, 124, 24, 154, 150, 237, 0, 184, 6, 209, 14, 215,
                    180, 238, 128,
                ],
            ),
            (
                "0x7963685967117ffb6fd019663dc9e782ebb1234a38501bffc2eb5380f8dc303b",
                vec![
                    211, 128, 143, 192, 151, 206, 123, 201, 7, 21, 179, 75, 159, 16, 0, 0, 0, 0,
                    128, 128,
                ],
            ),
            (
                "0x79afb7a5ffe6ccd537f9adff8287b78f75c37d97ea8a4dd504a08bc09926c3fa",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x7a08bb8417e6b18da3ba926568f1022c15553b2b0f1a32f2fd9e5a605469e54f",
                vec![
                    228, 1, 128, 160, 56, 91, 132, 210, 112, 89, 163, 199, 142, 126, 166, 58, 105,
                    30, 235, 156, 83, 118, 247, 122, 241, 19, 54, 118, 47, 140, 24, 136, 47, 247,
                    71, 26, 128,
                ],
            ),
            (
                "0x7a2464bc24d90557940e93a3b73308ea354ed7d988be720c545974a17959f93f",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0x7a3870cc1ed4fc29e9ab4dd3218dbb239dd32c9bf05bff03e325b7ba68486c47",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0x7bac5af423cb5e417fa6c103c7cb9777e80660ce3735ca830c238b0d41610186",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x7bff1b6b56891e66584ced453d09450c2fed9453b1644e8509bef9f9dd081bbb",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0x7c1edabb98857d64572f03c64ac803e4a14b1698fccffffd51675d99ee3ba217",
                vec![
                    228, 1, 128, 160, 97, 23, 109, 188, 5, 168, 83, 125, 141, 232, 95, 130, 160,
                    59, 142, 16, 73, 206, 167, 173, 10, 159, 14, 91, 96, 238, 21, 252, 166, 254,
                    13, 66, 128,
                ],
            ),
            (
                "0x7c3e44534b1398abc786e4591364c329e976dbde3b3ed3a4d55589de84bcb9a6",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x7c463797c90e9ba42b45ae061ffaa6bbd0dad48bb4998f761e81859f2a904a49",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0x7c48e400de1f24b4de94c59068fcd91a028576d13a22f900a7fcbd8f4845bcf4",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0x7c608293e741d1eb5ae6916c249a87b6540cf0c2369e96d293b1a7b5b9bd8b31",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0x7e1ef9f8d2fa6d4f8e6717c3dcccff352ea9b8b46b57f6106cdbeed109441799",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x7e839d9fd8a767e90a8b2f48a571f111dd2451bc5910cf2bf3ae79963e47e34d",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0x7f9726a7b2f5f3a501b2d7b18ec726f25f22c86348fae0f459d882ec5fd7d0c7",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x80a2c1f38f8e2721079a0de39f187adedcb81b2ab5ae718ec1b8d64e4aa6930e",
                vec![
                    228, 1, 128, 160, 45, 168, 110, 179, 212, 255, 221, 137, 81, 112, 188, 126,
                    240, 43, 105, 161, 22, 254, 33, 172, 44, 228, 90, 62, 216, 224, 187, 138, 241,
                    124, 249, 43, 128,
                ],
            ),
            (
                "0x80cd4a7b601d4ba0cb09e527a246c2b5dd25b6dbf862ac4e87c6b189bfce82d7",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0x81c0c51e15c9679ef12d02729c09db84220ba007efe7ced37a57132f6f0e83c9",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0x84c7ee50e102d0abf5750e781c1635d60346f20ab0d5e5f9830db1a592c658ff",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0x8510660ad5e3d35a30d4fb7c2615c040f9f698faae2ac48022e366deaeecbe77",
                vec![
                    228, 1, 128, 160, 39, 233, 182, 165, 76, 240, 251, 24, 132, 153, 197, 8, 189,
                    150, 212, 80, 148, 108, 214, 186, 28, 247, 108, 245, 52, 59, 92, 116, 69, 15,
                    102, 144, 128,
                ],
            ),
            (
                "0x8678559b30b321b0f0420a4a3e8cecfde90c6e56766b78c1723062c93c1f041f",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0x867bc89cf8d5b39f1712fbc77414bbd93012af454c226dcee0fb34ccc0017498",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0x86a73e3c668eb065ecac3402c6dc912e8eb886788ea147c770f119dcd30780c6",
                vec![
                    228, 1, 128, 160, 165, 169, 28, 249, 232, 21, 251, 85, 223, 20, 179, 238, 140,
                    19, 37, 169, 136, 203, 59, 109, 211, 71, 150, 201, 1, 56, 92, 60, 194, 153, 32,
                    115, 128,
                ],
            ),
            (
                "0x86d03d0f6bed220d046a4712ec4f451583b276df1aed33f96495d22569dc3485",
                vec![
                    228, 1, 128, 160, 226, 161, 100, 226, 195, 12, 243, 3, 145, 200, 143, 243, 42,
                    14, 32, 33, 148, 176, 143, 42, 97, 169, 205, 41, 39, 234, 94, 214, 223, 191,
                    16, 86, 128,
                ],
            ),
            (
                "0x873429def7829ff8227e4ef554591291907892fc8f3a1a0667dada3dc2a3eb84",
                vec![
                    228, 1, 128, 160, 84, 171, 205, 188, 139, 4, 188, 155, 112, 233, 189, 70, 203,
                    157, 185, 184, 235, 8, 207, 212, 173, 219, 164, 201, 65, 218, 204, 52, 221, 40,
                    100, 142, 128,
                ],
            ),
            (
                "0x878040f46b1b4a065e6b82abd35421eb69eededc0c9598b82e3587ae47c8a651",
                vec![
                    211, 128, 143, 192, 151, 206, 123, 201, 7, 21, 179, 75, 159, 16, 0, 0, 0, 0,
                    128, 128,
                ],
            ),
            (
                "0x87e33f70e1dd3c6ff68e3b71757d697fbeb20daae7a3cc8a7b1b3aa894592c50",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x88a5635dabc83e4e021167be484b62cbed0ecdaa9ac282dab2cd9405e97ed602",
                vec![
                    228, 1, 128, 160, 137, 189, 232, 157, 247, 242, 216, 51, 68, 165, 3, 148, 75,
                    179, 71, 184, 71, 242, 8, 223, 131, 114, 40, 187, 44, 223, 214, 195, 34, 140,
                    163, 223, 128,
                ],
            ),
            (
                "0x88bf4121c2d189670cb4d0a16e68bdf06246034fd0a59d0d46fb5cec0209831e",
                vec![
                    228, 1, 128, 160, 89, 115, 155, 163, 177, 86, 235, 120, 248, 187, 177, 75, 191,
                    61, 172, 222, 191, 222, 149, 20, 15, 88, 109, 182, 111, 114, 227, 17, 123, 148,
                    187, 103, 128,
                ],
            ),
            (
                "0x8989651e80c20af78b37fdb693d74ecafc9239426ff1315e1fb7b674dcdbdb75",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x8a8266874b43f78d4097f27b2842132faed7e7e430469eec7354541eb97c3ea0",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0x8b76305d3f00d33f77bd41496b4144fd3d113a2ec032983bd5830a8b73f61cf0",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x8c7bfaa19ea367dec5272872114c46802724a27d9b67ea3eed85431df664664e",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0x8e11480987056c309d7064ebbd887f086d815353cdbaadb796891ed25f8dcf61",
                vec![
                    211, 128, 143, 192, 151, 206, 123, 201, 7, 21, 179, 75, 159, 16, 0, 0, 0, 0,
                    128, 128,
                ],
            ),
            (
                "0x8ee17a1ec4bae15d8650323b996c55d5fa11a14ceec17ff1d77d725183904914",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x903f24b3d3d45bc50c082b2e71c7339c7060f633f868db2065ef611885abe37e",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0x910fb8b22867289cb57531ad39070ef8dbdbbe7aee941886a0e9f572b63ae9ee",
                vec![
                    228, 1, 128, 160, 115, 191, 252, 104, 169, 71, 250, 25, 183, 190, 205, 69, 102,
                    29, 34, 200, 112, 250, 200, 219, 242, 178, 87, 3, 225, 189, 171, 83, 103, 242,
                    149, 67, 128,
                ],
            ),
            (
                "0x913e2a02a28d71d595d7216a12311f6921a4caf40aeabf0f28edf937f1df72b4",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x92b13a73440c6421da22e848d23f9af80610085ab05662437d850c97a012d8d3",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x92d0f0954f4ec68bd32163a2bd7bc69f933c7cdbfc6f3d2457e065f841666b1c",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0x93843d6fa1fe5709a3035573f61cc06832f0377544d16d3a0725e78a0fa0267c",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0x943f42ad91e8019f75695946d491bb95729f0dfc5dbbb953a7239ac73f208943",
                vec![
                    228, 1, 128, 160, 169, 88, 1, 9, 190, 47, 125, 53, 181, 54, 0, 80, 194, 206,
                    215, 78, 93, 77, 234, 47, 130, 212, 110, 141, 38, 110, 216, 145, 87, 99, 96, 4,
                    128,
                ],
            ),
            (
                "0x946bfb429d90f1b39bb47ada75376a8d90a5778068027d4b8b8514ac13f53eca",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0x961508ac3c93b30ee9a5a34a862c9fe1659e570546ac6c2e35da20f6d2bb5393",
                vec![
                    228, 1, 128, 160, 217, 26, 207, 48, 89, 52, 166, 12, 150, 10, 147, 251, 0, 249,
                    39, 236, 121, 48, 139, 138, 145, 157, 36, 73, 250, 237, 231, 34, 194, 50, 76,
                    179, 128,
                ],
            ),
            (
                "0x96c43ef9dce3410b78df97be69e7ccef8ed40d6e5bfe6582ea4cd7d577aa4569",
                vec![
                    228, 1, 128, 160, 90, 130, 175, 241, 38, 255, 235, 255, 118, 0, 43, 30, 77,
                    224, 60, 64, 186, 73, 75, 129, 203, 63, 188, 82, 143, 35, 228, 190, 53, 169,
                    175, 230, 128,
                ],
            ),
            (
                "0x96d7104053877823b058fd9248e0bba2a540328e52ffad9bb18805e89ff579dc",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0x974a4800ec4c0e998f581c6ee8c3972530989e97a179c6b2d40b8710c036e7b1",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0x97b25febb46f44607c87a3498088c605086df207c7ddcd8ee718836a516a9153",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0x97f72ff641eb40ee1f1163544931635acb7550a0d44bfb9f4cc3aeae829b6d7d",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0x98bb9ba48fda7bb8091271ab0e53d7e0022fb1f1fa8fa00814e193c7d4b91eb3",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x9966a8b4cd856b175855258fa7e412ffef06d9e92b519050fa7ac06d8952ac84",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x99ce1680f73f2adfa8e6bed135baa3360e3d17f185521918f9341fc236526321",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0x99dba7e9230d5151cc37ff592fa1592f27c7c81d203760dfaf62ddc9f3a6b8fd",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x99e56541f21039c9b7c63655333841a3415de0d27b79d18ade9ec7ecde7a1139",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0x9a1896e612ca43ecb7601990af0c3bc135b9012c50d132769dfb75d0038cc3be",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x9d42947ac5e61285567f65d4b400d90343dbd3192534c4c1f9d941c04f48f17c",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0x9de451c4f48bdb56c6df198ff8e1f5e349a84a4dc11de924707718e6ac897aa6",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0x9fe8b6e43098a4df56e206d479c06480801485dfd8ec3da4ccc3cebf5fba89a1",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0x9feaf0bd45df0fbf327c964c243b2fbc2f0a3cb48fedfeea1ae87ac1e66bc02f",
                vec![
                    211, 128, 143, 192, 151, 206, 123, 201, 7, 21, 179, 75, 159, 16, 0, 0, 0, 0,
                    128, 128,
                ],
            ),
            (
                "0xa02abeb418f26179beafd96457bda8c690c6b1f3fbabac392d0920863edddbc6",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0xa02c8b02efb52fad3056fc96029467937c38c96d922250f6d2c0f77b923c85aa",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0xa03fe040e4264070290e95ffe06bf9da0006556091f17c5df5abaa041de0c2f7",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0xa0f5dc2d18608f8e522ffffd86828e3d792b36d924d5505c614383ddff9be2eb",
                vec![
                    228, 1, 128, 160, 42, 254, 147, 225, 176, 242, 110, 88, 141, 40, 9, 18, 126,
                    67, 96, 173, 126, 40, 207, 85, 36, 152, 178, 188, 72, 71, 214, 188, 218, 115,
                    140, 219, 128,
                ],
            ),
            (
                "0xa13bfef92e05edee891599aa5e447ff2baa1708d9a6473a04ef66ab94f2a11e4",
                vec![
                    211, 128, 143, 192, 151, 206, 123, 201, 7, 21, 179, 75, 159, 16, 0, 0, 0, 0,
                    128, 128,
                ],
            ),
            (
                "0xa15773c9bfabef49e9825460ed95bf67b22b67d7806c840e0eb546d73c424768",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0xa248850a2e0d6fe62259d33fc498203389fa754c3bd098163e86946888e455bd",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0xa3abdaefbb886078dc6c5c72e4bc8d12e117dbbd588236c3fa7e0c69420eb24a",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0xa3d8baf7ae7c96b1020753d12154e28cc7206402037c28c49c332a08cf7c4b51",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0xa5541b637a896d30688a80b7affda987d9597aac7ccd9799c15999a1d7d094e2",
                vec![
                    228, 1, 128, 160, 243, 144, 38, 74, 202, 241, 67, 60, 14, 166, 112, 178, 192,
                    148, 163, 0, 118, 100, 20, 105, 82, 74, 226, 79, 95, 221, 196, 78, 153, 197,
                    176, 50, 128,
                ],
            ),
            (
                "0xa601eb611972ca80636bc39087a1dae7be5a189b94bda392f84d6ce0d3c866b9",
                vec![
                    228, 1, 128, 160, 156, 50, 255, 213, 5, 145, 21, 187, 169, 174, 217, 23, 79,
                    90, 184, 180, 53, 46, 63, 81, 168, 93, 222, 51, 0, 15, 112, 60, 155, 159, 231,
                    194, 128,
                ],
            ),
            (
                "0xa683478d0c949580d5738b490fac8129275bb6e921dfe5eae37292be3ee281b9",
                vec![
                    228, 1, 128, 160, 193, 91, 67, 229, 244, 133, 62, 200, 218, 83, 235, 222, 3,
                    222, 135, 185, 74, 252, 228, 42, 156, 2, 246, 72, 173, 139, 219, 34, 70, 4,
                    196, 173, 128,
                ],
            ),
            (
                "0xa87387b50b481431c6ccdb9ae99a54d4dcdd4a3eff75d7b17b4818f7bbfc21e9",
                vec![
                    228, 1, 128, 160, 226, 167, 47, 91, 251, 235, 167, 15, 201, 171, 80, 98, 55,
                    186, 39, 192, 150, 164, 233, 108, 57, 104, 202, 191, 91, 27, 47, 181, 68, 49,
                    181, 207, 128,
                ],
            ),
            (
                "0xa9233a729f0468c9c309c48b82934c99ba1fd18447947b3bc0621adb7a5fc643",
                vec![
                    211, 128, 143, 192, 151, 206, 123, 201, 7, 21, 179, 75, 159, 16, 0, 0, 0, 0,
                    128, 128,
                ],
            ),
            (
                "0xa95c88d7dc0f2373287c3b2407ba8e7419063833c424b06d8bb3b29181bb632e",
                vec![196, 128, 128, 128, 128],
            ),
            (
                "0xa9656c0192bb27f0ef3f93ecc6cc990dd146da97ac11f3d8d0899fba68d5749a",
                vec![
                    228, 1, 128, 160, 114, 23, 203, 116, 112, 84, 48, 111, 130, 110, 120, 170, 63,
                    198, 143, 228, 68, 18, 153, 163, 55, 236, 234, 29, 98, 88, 47, 45, 168, 167,
                    243, 54, 128,
                ],
            ),
            (
                "0xa9970b3744a0e46b248aaf080a001441d24175b5534ad80755661d271b976d67",
                vec![
                    228, 1, 128, 160, 18, 222, 69, 68, 100, 15, 200, 160, 39, 225, 169, 18, 215,
                    118, 185, 6, 117, 190, 191, 213, 7, 16, 194, 135, 107, 42, 36, 236, 158, 206,
                    211, 103, 128,
                ],
            ),
            (
                "0xa9de128e7d4347403eb97f45e969cd1882dfe22c1abe8857aab3af6d0f9e9b92",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0xa9fd2e3a6de5a9da5badd719bd6e048acefa6d29399d8a99e19fd9626805b60b",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0xaa0ac2f707a3dc131374839d4ee969eeb1cb55adea878f56e7b5b83d187d925c",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0xaa0ffaa57269b865dccce764bf412de1dff3e7bba22ce319ef09e5907317b3e7",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0xaad7b91d085a94c11a2f7e77dc95cfcfc5daf4f509ca4e0c0e493b86c6cbff78",
                vec![
                    228, 1, 128, 160, 160, 144, 182, 111, 188, 164, 108, 183, 26, 189, 29, 170,
                    141, 65, 157, 44, 110, 41, 16, 148, 245, 40, 114, 151, 141, 252, 177, 195, 26,
                    215, 169, 0, 128,
                ],
            ),
            (
                "0xab7bdc41a80ae9c8fcb9426ba716d8d47e523f94ffb4b9823512d259c9eca8cd",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0xabd8afe9fbf5eaa36c506d7c8a2d48a35d013472f8182816be9c833be35e50da",
                vec![
                    211, 128, 143, 192, 151, 206, 123, 201, 7, 21, 179, 75, 159, 16, 0, 0, 0, 0,
                    128, 128,
                ],
            ),
            (
                "0xabdc44a9bc7ccf1ce76b942d25cd9d731425cd04989597d7a2e36423e2dac7ee",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0xac7183ebb421005a660509b070d3d47fc4e134cb7379c31dc35dc03ebd02e1cf",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0xad6a4a6ebd5166c9b5cc8cfbaec176cced40fa88c73d83c67f0c3ed426121ebc",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0xad99b5bc38016547d5859f96be59bf18f994314116454def33ebfe9a892c508a",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0xae88076d02b19c4d09cb13fca14303687417b632444f3e30fc4880c225867be3",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0xaeaf19d38b69be4fb41cc89e4888708daa6b9b1c3f519fa28fe9a0da70cd8697",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0xaef83ad0ab332330a20e88cd3b5a4bcf6ac6c175ee780ed4183d11340df17833",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0xaf38e0e6a4a4005507b5d3e9470e8ccc0273b74b6971f768cbdf85abeab8a95b",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0xaf7c37d08a73483eff9ef5054477fb5d836a184aa07c3edb4409b9eb22dd56ca",
                vec![
                    228, 1, 128, 160, 197, 118, 4, 164, 97, 201, 78, 205, 172, 18, 219, 183, 6,
                    165, 43, 50, 145, 61, 114, 37, 59, 175, 251, 137, 6, 231, 66, 114, 74, 225, 36,
                    73, 128,
                ],
            ),
            (
                "0xb062c716d86a832649bccd53e9b11c77fc8a2a00ef0cc0dd2f561688a69d54f7",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0xb17ea61d092bd5d77edd9d5214e9483607689cdcc35a30f7ea49071b3be88c64",
                vec![
                    211, 128, 143, 192, 151, 206, 123, 201, 7, 21, 179, 75, 159, 16, 0, 0, 0, 0,
                    128, 128,
                ],
            ),
            (
                "0xb1b2c1c59637202bb0e0d21255e44e0df719fe990be05f213b1b813e3d8179d7",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0xb1b2fd7758f73e25a2f9e72edde82995b2b32ab798bcffd2c7143f2fc8196fd8",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0xb31919583a759b75e83c14d00d0a89bb36adc452f73cee2933a346ccebaa8e31",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0xb3a33a7f35ca5d08552516f58e9f76219716f9930a3a11ce9ae5db3e7a81445d",
                vec![
                    228, 1, 128, 160, 131, 71, 24, 17, 17, 33, 226, 5, 143, 219, 144, 165, 31, 68,
                    128, 40, 7, 24, 87, 225, 31, 189, 85, 212, 50, 86, 23, 77, 245, 106, 240, 26,
                    128,
                ],
            ),
            (
                "0xb40cc623b26a22203675787ca05b3be2c2af34b6b565bab95d43e7057e458684",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0xb4f179efc346197df9c3a1cb3e95ea743ddde97c27b31ad472d352dba09ee1f5",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0xb58e22a9ece8f9b3fdbaa7d17fe5fc92345df11d6863db4159647d64a34ff10b",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0xb58e67c536550fdf7140c8333ca62128df469a7270b16d528bc778909e0ac9a5",
                vec![
                    228, 1, 128, 160, 35, 168, 136, 192, 164, 100, 206, 70, 22, 81, 252, 27, 226,
                    207, 160, 203, 107, 164, 209, 177, 37, 171, 229, 180, 71, 238, 173, 249, 197,
                    173, 241, 241, 128,
                ],
            ),
            (
                "0xb5bca5e9ccef948c2431372315acc3b96e098d0e962b0c99d634a0475b670dc3",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0xb66092bc3624d84ff94ee42b097e846baf6142197d2c31245734d56a275c8eb9",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0xb7c2ef96238f635f86f9950700e36368efaaa70e764865dddc43ff6e96f6b346",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0xb7d9d175039df1ba52c734547844f8805252893c029f7dbba9a63f8bce3ee306",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0xb888c9946a84be90a9e77539b5ac68a3c459761950a460f3e671b708bb39c41f",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0xb8d9b988ed60dbf5dca3e9d169343ca667498605f34fb6c30b45b2ed0f996f1a",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0xb91824b28183c95881ada12404d5ee8af8123689a98054d41aaf4dd5bec50e90",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0xb9400acf38453fd206bc18f67ba04f55b807b20e4efc2157909d91d3a9f7bed2",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0xb990eaca858ea15fda296f3f47baa2939e8aa8bbccc12ca0c3746d9b5d5fb2ae",
                vec![
                    228, 1, 128, 160, 137, 236, 176, 206, 238, 162, 12, 205, 125, 27, 24, 207, 29,
                    53, 183, 162, 253, 123, 118, 221, 200, 214, 39, 244, 51, 4, 237, 139, 49, 176,
                    18, 72, 128,
                ],
            ),
            (
                "0xb9cddc73dfdacd009e55f27bdfd1cd37eef022ded5ce686ab0ffe890e6bf311e",
                vec![
                    228, 1, 128, 160, 61, 32, 254, 221, 39, 11, 55, 113, 112, 111, 224, 10, 88, 10,
                    21, 84, 57, 190, 87, 232, 213, 80, 118, 45, 239, 16, 144, 110, 131, 237, 88,
                    187, 128,
                ],
            ),
            (
                "0xba1d0afdfee510e8852f24dff964afd824bf36d458cf5f5d45f02f04b7c0b35d",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0xbaae09901e990935de19456ac6a6c8bc1e339d0b80ca129b8622d989b5c79120",
                vec![
                    228, 1, 128, 160, 37, 180, 46, 197, 72, 8, 67, 160, 50, 140, 99, 188, 80, 239,
                    248, 89, 93, 144, 241, 209, 176, 175, 202, 178, 244, 161, 155, 136, 140, 121,
                    79, 55, 128,
                ],
            ),
            (
                "0xbb861b82d884a70666afeb78bbf30cab7fdccf838f4d5ce5f4e5ca1be6be61b1",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0xbbdc59572cc62c338fb6e027ab00c57cdeed233c8732680a56a5747141d20c7c",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0xbccd3d2f920dfb8d70a38c9ccd5ed68c2ef6e3372199381767ce222f13f36c87",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0xbccd85b63dba6300f84c561c5f52ce08a240564421e382e6f550ce0c12f2f632",
                vec![
                    228, 1, 128, 160, 234, 131, 56, 147, 131, 21, 34, 112, 16, 64, 147, 237, 93,
                    254, 52, 186, 64, 60, 117, 48, 129, 51, 170, 27, 232, 245, 26, 216, 4, 179,
                    233, 238, 128,
                ],
            ),
            (
                "0xbcebc35bfc663ecd6d4410ee2363e5b7741ee953c7d3359aa585095e503d20c8",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0xbe7d987a9265c0e44e9c5736fb2eb38c41973ce96e5e8e6c3c713f9d50a079ff",
                vec![
                    228, 1, 128, 160, 175, 213, 78, 129, 243, 228, 21, 64, 127, 8, 18, 166, 120,
                    133, 111, 27, 64, 104, 237, 100, 160, 139, 63, 59, 245, 178, 25, 15, 207, 178,
                    50, 45, 128,
                ],
            ),
            (
                "0xbea55c1dc9f4a9fb50cbedc70448a4e162792b9502bb28b936c7e0a2fd7fe41d",
                vec![
                    228, 1, 128, 160, 49, 10, 42, 200, 61, 126, 62, 77, 51, 49, 2, 177, 247, 21,
                    59, 176, 65, 107, 56, 66, 126, 178, 227, 53, 220, 102, 50, 215, 121, 168, 180,
                    175, 128,
                ],
            ),
            (
                "0xbf632670b6fa18a8ad174a36180202bfef9a92c2eeda55412460491ae0f6a969",
                vec![
                    228, 1, 128, 160, 207, 33, 35, 209, 16, 153, 127, 66, 104, 33, 211, 229, 65,
                    51, 78, 67, 253, 214, 181, 40, 108, 60, 51, 37, 44, 36, 181, 248, 170, 252,
                    122, 162, 128,
                ],
            ),
            (
                "0xbfaac98225451c56b2f9aec858cffc1eb253909615f3d9617627c793b938694f",
                vec![
                    228, 1, 128, 160, 238, 152, 33, 98, 26, 165, 236, 154, 183, 213, 135, 139, 42,
                    153, 82, 40, 173, 205, 202, 203, 113, 13, 245, 34, 210, 249, 27, 67, 77, 59,
                    220, 121, 128,
                ],
            ),
            (
                "0xbfe5dee42bddd2860a8ebbcdd09f9c52a588ba38659cf5e74b07d20f396e04d4",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0xbfe731f071443795cef55325f32e6e03c8c0d0398671548dfd5bc96b5a6555c0",
                vec![
                    228, 1, 128, 160, 178, 95, 158, 79, 111, 145, 58, 74, 30, 141, 235, 247, 212,
                    117, 43, 250, 82, 29, 20, 123, 182, 124, 105, 213, 133, 83, 1, 231, 109, 216,
                    6, 51, 128,
                ],
            ),
            (
                "0xc0ce77c6a355e57b89cca643e70450612c0744c9f0f8bf7dee51d6633dc850b1",
                vec![
                    228, 1, 128, 160, 223, 60, 27, 250, 184, 247, 231, 10, 142, 223, 148, 121, 47,
                    145, 228, 182, 178, 194, 170, 97, 202, 246, 135, 228, 246, 203, 104, 157, 24,
                    10, 219, 128, 128,
                ],
            ),
            (
                "0xc13c19f53ce8b6411d6cdaafd8480dfa462ffdf39e2eb68df90181a128d88992",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0xc157e0d637d64b90e2c59bc8bed2acd75696ea1ac6b633661c12ce8f2bce0d62",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0xc192ea2d2bb89e9bb7f17f3a282ebe8d1dd672355b5555f516b99b91799b01f6",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0xc1a6a0bf60ee7b3228ecf6cb7c9e5491fbf62642a3650d73314e976d9eb9a966",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0xc2406cbd93e511ef493ac81ebe2b6a3fbecd05a3ba52d82a23a88eeb9d8604f0",
                vec![
                    228, 1, 128, 160, 130, 179, 38, 100, 24, 37, 55, 143, 170, 17, 198, 65, 201,
                    22, 242, 226, 44, 1, 8, 15, 72, 125, 224, 70, 62, 48, 213, 227, 43, 150, 15,
                    151, 128,
                ],
            ),
            (
                "0xc250f30c01f4b7910c2eb8cdcd697cf493f6417bb2ed61d637d625a85a400912",
                vec![
                    228, 1, 128, 160, 202, 57, 245, 244, 238, 60, 107, 51, 239, 231, 188, 72, 84,
                    57, 249, 127, 157, 198, 47, 101, 133, 44, 122, 28, 223, 84, 250, 177, 227, 183,
                    4, 41, 128,
                ],
            ),
            (
                "0xc251a3acb75a90ff0cdca31da1408a27ef7dcaa42f18e648f2be1a28b35eac32",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0xc2c26fbc0b7893d872fa528d6c235caab9164feb5b54c48381ff3d82c8244e77",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0xc3791fc487a84f3731eb5a8129a7e26f357089971657813b48a821f5582514b3",
                vec![
                    211, 128, 143, 192, 151, 206, 123, 201, 7, 21, 179, 75, 159, 16, 0, 0, 0, 0,
                    128, 128,
                ],
            ),
            (
                "0xc3ac56e9e7f2f2c2c089e966d1b83414951586c3afeb86300531dfa350e38929",
                vec![
                    228, 1, 128, 160, 129, 142, 175, 90, 219, 86, 198, 114, 136, 137, 186, 102,
                    182, 152, 12, 214, 107, 65, 25, 159, 0, 7, 205, 217, 5, 174, 115, 148, 5, 227,
                    198, 48, 128,
                ],
            ),
            (
                "0xc3c8e2dc64e67baa83b844263fe31bfe24de17bb72bfed790ab345b97b007816",
                vec![
                    211, 128, 143, 192, 151, 206, 123, 201, 7, 21, 179, 75, 159, 16, 0, 0, 0, 0,
                    128, 128,
                ],
            ),
            (
                "0xc4bab059ee8f7b36c82ada44d22129671d8f47f254ca6a48fded94a8ff591c88",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0xc54ffffcbaa5b566a7cf37386c4ce5a338d558612343caaa99788343d516aa5f",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0xc781c7c3babeb06adfe8f09ecb61dbe0eb671e41f3a1163faac82fdfa2bc83e8",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0xc7fc033fe9f00d24cb9c479ddc0598e592737c305263d088001d7419d16feffa",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0xc9ea69dc9e84712b1349c9b271956cc0cb9473106be92d7a937b29e78e7e970e",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0xca7ad42d3c4fe14ddb81bf27d4679725a1f6c3f23b688681bb6f24262d63212f",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0xcac96145454c46255fccca35343d9505164dabe319c17d81fda93cf1171e4c6e",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0xcade985c7fb6d371d0c7f7cb40178e7873d623eadcc37545798ec33a04bb2173",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0xcb54add475a18ea02ab1adf9e2e73da7f23ecd3e92c4fa8ca4e8f588258cb5d3",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0xcb6f450b4720c6b36d3a12271e35ace27f1d527d46b073771541ad39cc59398d",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0xcc74930e1ee0e71a8081f247ec47442a3e5d00897966754a5b3ee8beb2c1160c",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0xcd07379b0120ad9a9c7fa47e77190be321ab107670f3115fec485bebb467307d",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0xcd6b3739d4dbce17dafc156790f2a3936eb75ce95e9bba039dd76661f40ea309",
                vec![
                    228, 1, 128, 160, 176, 112, 15, 225, 61, 186, 249, 75, 229, 11, 203, 236, 19,
                    167, 181, 62, 108, 186, 3, 75, 41, 163, 218, 186, 152, 250, 134, 31, 88, 151,
                    33, 63, 128,
                ],
            ),
            (
                "0xce732a5e3b88ae26790aeb390a2bc02c449fdf57665c6d2c2b0dbce338c4377e",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0xd1691564c6a5ab1391f0495634e749b9782de33756b6a058f4a9536c1b37bca6",
                vec![
                    228, 1, 128, 160, 214, 14, 228, 173, 90, 187, 231, 89, 98, 47, 202, 92, 83, 97,
                    9, 177, 30, 133, 170, 43, 72, 192, 190, 42, 235, 240, 29, 245, 151, 231, 77,
                    186, 128,
                ],
            ),
            (
                "0xd16e029e8c67c3f330cddaa86f82d31f523028404dfccd16d288645d718eb9da",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0xd2501ae11a14bf0c2283a24b7e77c846c00a63e71908c6a5e1caff201bad0762",
                vec![
                    228, 128, 128, 160, 73, 27, 44, 251, 169, 118, 178, 231, 139, 217, 190, 59,
                    193, 92, 153, 100, 146, 114, 5, 252, 52, 201, 149, 74, 77, 97, 187, 232, 23,
                    11, 165, 51, 128,
                ],
            ),
            (
                "0xd2f394b4549b085fb9b9a8b313a874ea660808a4323ab2598ee15ddd1eb7e897",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0xd3443fa37ee617edc09a9c930be4873c21af2c47c99601d5e20483ce6d01960a",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0xd352b05571154d9a2061143fe6df190a740a2d321c59eb94a54acb7f3054e489",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0xd37b6f5e5f0fa6a1b3fd15c9b3cf0fb595ba245ab912ad8059e672fa55f061b8",
                vec![
                    228, 1, 128, 160, 89, 147, 108, 21, 196, 84, 147, 62, 188, 73, 137, 175, 167,
                    126, 53, 15, 118, 64, 48, 27, 7, 52, 26, 234, 213, 241, 178, 102, 142, 235, 29,
                    173, 128,
                ],
            ),
            (
                "0xd52564daf6d32a6ae29470732726859261f5a7409b4858101bd233ed5cc2f662",
                vec![
                    211, 128, 143, 192, 151, 206, 123, 201, 7, 21, 179, 75, 159, 16, 0, 0, 0, 0,
                    128, 128,
                ],
            ),
            (
                "0xd57eafe6d4c5b91fe7114e199318ab640e55d67a1e9e3c7833253808b7dca75f",
                vec![
                    228, 1, 128, 160, 224, 163, 211, 184, 57, 252, 160, 245, 71, 69, 208, 197, 10,
                    4, 142, 66, 76, 146, 89, 240, 99, 183, 65, 100, 16, 164, 66, 46, 235, 127, 131,
                    126, 128,
                ],
            ),
            (
                "0xd5e252ab2fba10107258010f154445cf7dffc42b7d8c5476de9a7adb533d73f1",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0xd5e5e7be8a61bb5bfa271dfc265aa9744dea85de957b6cffff0ecb403f9697db",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0xd623b1845175b206c127c08046281c013e4a3316402a771f1b3b77a9831143f5",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0xd63070208c85e91c4c8c942cf52c416f0f3004c392a15f579350168f178dba2e",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0xd72e318c1cea7baf503950c9b1bd67cf7caf2f663061fcde48d379047a38d075",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0xd8489fd0ce5e1806b24d1a7ce0e4ba8f0856b87696456539fcbb625a9bed2ccc",
                vec![
                    228, 1, 128, 160, 52, 55, 128, 49, 1, 168, 4, 10, 202, 39, 63, 183, 52, 215,
                    150, 90, 135, 248, 35, 255, 30, 247, 140, 126, 220, 170, 211, 88, 235, 152,
                    222, 227, 128,
                ],
            ),
            (
                "0xd84f7711be2f8eca69c742153230995afb483855b7c555b08da330139cdb9579",
                vec![
                    228, 1, 128, 160, 158, 83, 240, 162, 221, 180, 48, 210, 127, 111, 255, 160,
                    166, 139, 95, 117, 219, 29, 104, 226, 65, 19, 220, 202, 110, 51, 145, 140, 218,
                    232, 8, 70, 128,
                ],
            ),
            (
                "0xd9f987fec216556304eba05bcdae47bb736eea5a4183eb3e2c3a5045734ae8c7",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0xd9fa858992bc92386a7cebcd748eedd602bf432cb4b31607566bc92b85179624",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0xda81833ff053aff243d305449775c3fb1bd7f62c4a3c95dc9fb91b85e032faee",
                vec![
                    211, 128, 143, 192, 151, 206, 123, 201, 7, 21, 179, 75, 159, 16, 0, 0, 0, 0,
                    128, 128,
                ],
            ),
            (
                "0xdbd66b6a89e01c76ae5f8cb0dcd8a24e787f58f015c9b08972bfabefa2eae0d5",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0xdbea1fd70fe1c93dfef412ce5d8565d87d6843aac044d3a015fc3db4d20a351b",
                vec![
                    228, 1, 128, 160, 190, 254, 85, 182, 6, 168, 101, 195, 137, 142, 194, 9, 59,
                    209, 96, 179, 124, 57, 118, 1, 21, 22, 244, 55, 54, 202, 194, 169, 167, 236,
                    212, 202, 128,
                ],
            ),
            (
                "0xdc9ea08bdea052acab7c990edbb85551f2af3e1f1a236356ab345ac5bcc84562",
                vec![
                    228, 128, 128, 160, 32, 127, 108, 62, 69, 5, 70, 176, 209, 243, 188, 106, 111,
                    175, 91, 250, 11, 255, 128, 57, 108, 85, 213, 103, 184, 52, 207, 14, 124, 118,
                    3, 71, 128,
                ],
            ),
            (
                "0xdcda5b5203c2257997a574bdf85b2bea6d04829e8d7e048a709badc0fb99288c",
                vec![
                    228, 1, 128, 160, 174, 68, 1, 67, 210, 30, 36, 169, 49, 182, 117, 111, 107, 61,
                    80, 211, 55, 234, 240, 219, 62, 108, 52, 227, 106, 180, 111, 226, 217, 158,
                    248, 62, 128,
                ],
            ),
            (
                "0xdce547cc70c79575ef72c061502d6066db1cbce200bd904d5d2b20d4f1cb5963",
                vec![
                    228, 1, 128, 160, 38, 37, 248, 162, 61, 36, 165, 223, 246, 167, 159, 99, 43,
                    16, 32, 89, 51, 98, 166, 172, 98, 47, 165, 35, 116, 96, 188, 103, 176, 170, 14,
                    211, 128,
                ],
            ),
            (
                "0xdd1589b1fe1d9b4ca947f98ff324de7887af299d5490ed92ae40e95eec944118",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0xdef989cb85107747de11222bd7418411f8f3264855e1939ef6bef9447e42076d",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0xe02ec497b66cb57679eb01de1bed2ad385a3d18130441a9d337bd14897e85d39",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0xe04fdefc4f2eefd22721d5944411b282d0fcb1f9ac218f54793a35bca8199c25",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0xe09e5f27b8a7bf61805df6e5fefc24eb6894281550c2d06250adecfe1e6581d7",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0xe0c5acf66bda927704953fdf7fb4b99e116857121c069eca7fb9bd8acfc25434",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0xe1068e9986da7636501d8893f67aa94f5d73df849feab36505fd990e2d6240e9",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0xe1b86a365b0f1583a07fc014602efc3f7dedfa90c66e738e9850719d34ac194e",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0xe1eb1e18ae510d0066d60db5c2752e8c33604d4da24c38d2bda07c0cb6ad19e4",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0xe31747e6542bf4351087edfbeb23e225e4217b5fa25d385f33cd024df0c9ae12",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0xe333845edc60ed469a894c43ed8c06ec807dafd079b3c948077da56e18436290",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0xe3c2e12be28e2e36dc852e76dd32e091954f99f2a6480853cd7b9e01ec6cd889",
                vec![
                    228, 1, 128, 160, 204, 72, 248, 209, 192, 221, 110, 200, 171, 123, 189, 121,
                    45, 148, 246, 167, 76, 136, 118, 180, 27, 200, 89, 206, 226, 34, 142, 141, 173,
                    130, 7, 164, 128,
                ],
            ),
            (
                "0xe3c79e424fd3a7e5bf8e0426383abd518604272fda87ecd94e1633d36f55bbb6",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0xe3d7213321be060ae2e1ff70871131ab3e4c9f4214a17fe9441453745c29365b",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0xe42a85d04a1d0d9fe0703020ef98fa89ecdeb241a48de2db73f2feeaa2e49b0f",
                vec![
                    228, 1, 128, 160, 251, 0, 114, 154, 95, 79, 154, 36, 54, 185, 153, 170, 113,
                    89, 73, 122, 156, 216, 141, 21, 87, 112, 248, 115, 168, 24, 181, 80, 82, 197,
                    240, 103, 128,
                ],
            ),
            (
                "0xe4d9c31cc9b4a9050bbbf77cc08ac26d134253dcb6fd994275c5c3468f5b7810",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0xe5302e42ca6111d3515cbbb2225265077da41d997f069a6c492fa3fcb0fdf284",
                vec![
                    211, 128, 143, 192, 151, 206, 123, 201, 7, 21, 179, 75, 159, 16, 0, 0, 0, 0,
                    128, 128,
                ],
            ),
            (
                "0xe6388bfcbbd6000e90a10633c72c43b0b0fed7cf38eab785a71e6f0c5b80a26a",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0xe69f40f00148bf0d4dfa28b3f3f5a0297790555eca01a00e49517c6645096a6c",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0xe6c5edf6a0fbdcff100e5ceafb63cba9aea355ba397a93fdb42a1a67b91375f8",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0xe6d72f72fd2fc8af227f75ab3ab199f12dfb939bdcff5f0acdac06a90084def8",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0xe73b3367629c8cb991f244ac073c0863ad1d8d88c2e180dd582cefda2de4415e",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0xe74ac72f03e8c514c2c75f3c4f54ba31e920374ea7744ef1c33937e64c7d54f1",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0xe7c6828e1fe8c586b263a81aafc9587d313c609c6db8665a42ae1267cd9ade59",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0xe99460a483f3369006e3edeb356b3653699f246ec71f30568617ebc702058f59",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0xea810ea64a420acfa917346a4a02580a50483890cba1d8d1d158d11f1c59ed02",
                vec![
                    228, 1, 128, 160, 147, 106, 198, 37, 24, 72, 218, 105, 161, 145, 204, 145, 23,
                    78, 75, 117, 131, 161, 42, 67, 216, 150, 226, 67, 132, 30, 169, 139, 101, 242,
                    100, 173, 128,
                ],
            ),
            (
                "0xeba984db32038d7f4d71859a9a2fc6e19dde2e23f34b7cedf0c4bf228c319f17",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0xec3e92967d10ac66eff64a5697258b8acf87e661962b2938a0edcd78788f360d",
                vec![
                    211, 128, 143, 192, 151, 206, 123, 201, 7, 21, 179, 75, 159, 16, 0, 0, 0, 0,
                    128, 128,
                ],
            ),
            (
                "0xed263a22f0e8be37bcc1873e589c54fe37fdde92902dc75d656997a7158a9d8c",
                vec![
                    228, 1, 128, 160, 229, 71, 192, 5, 2, 83, 7, 91, 27, 228, 33, 6, 8, 188, 99,
                    156, 255, 231, 1, 16, 25, 76, 49, 100, 129, 35, 94, 115, 139, 233, 97, 231,
                    128,
                ],
            ),
            (
                "0xedd9b1f966f1dfe50234523b479a45e95a1a8ec4a057ba5bfa7b69a13768197c",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0xee9186a01e5e1122b61223b0e6acc6a069c9dcdb7307b0a296421272275f821b",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0xefaff7acc3ad3417517b21a92187d2e63d7a77bc284290ed406d1bc07ab3d885",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0xf0877d51b7712e08f2a3c96cddf50ff61b8b90f80b8b9817ea613a8a157b0c45",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0xf0a51b55aadfa3cafdd214b0676816e574931a683f51218207c625375884e785",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0xf164775805f47d8970d3282188009d4d7a2da1574fe97e5d7bc9836a2eed1d5b",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0xf16522fc36907ee1e9948240b0c1d1d105a75cc63b71006f16c20d79ad469bd7",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0xf19ee923ed66b7b9264c2644aa20e5268a251b4914ca81b1dffee96ecb074cb1",
                vec![
                    228, 1, 128, 160, 205, 62, 117, 41, 158, 150, 125, 95, 136, 211, 6, 190, 144,
                    90, 19, 67, 67, 178, 36, 211, 253, 90, 134, 27, 26, 105, 13, 224, 226, 223,
                    225, 186, 128,
                ],
            ),
            (
                "0xf2b9bc1163840284f3eb15c539972edad583cda91946f344f4cb57be15af9c8f",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0xf33a7b66489679fa665dbfb4e6dd4b673495f853850eedc81d5f28bd2f4bd3b5",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0xf462aaa112b195c148974ff796a81c0e7f9a972d04e60c178ac109102d593a88",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0xf4a1c4554b186a354b3e0c467eef03df9907cd5a5d96086c1a542b9e5160ca78",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0xf63360f8bb23f88b0a564f9e07631c38c73b4074ba4192d6131336ef02ee9cf2",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0xf84223f460140ad56af9836cfa6c1c58c1397abf599c214689bc881066020ff7",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0xfab4c6889992a3f4e96b005dfd851021e9e1ec2631a7ccd2a001433e35077968",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0xfb2ab315988de92dcf6ba848e756676265b56e4b84778a2c955fb2b3c848c51c",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0xfb5a31c5cfd33dce2c80a30c5efc28e5f4025624adcc2205a2504a78c57bdd1c",
                vec![
                    228, 1, 128, 160, 73, 63, 144, 67, 84, 2, 223, 9, 7, 1, 155, 255, 198, 221, 37,
                    161, 124, 228, 172, 214, 235, 96, 119, 239, 148, 193, 98, 111, 13, 119, 201,
                    240, 128,
                ],
            ),
            (
                "0xfb9474d0e5538fcd99e8d8d024db335b4e057f4bcd359e85d78f4a5226b33272",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0xfc3d2e27841c0913d10aa11fc4af4793bf376efe3d90ce8360aa392d0ecefa24",
                vec![
                    228, 1, 128, 160, 123, 245, 66, 189, 175, 245, 191, 227, 211, 60, 38, 168, 135,
                    119, 119, 59, 94, 82, 84, 97, 9, 60, 54, 172, 176, 218, 181, 145, 163, 25, 229,
                    9, 128,
                ],
            ),
            (
                "0xfc4870c3cd21d694424c88f0f31f75b2426e1530fdea26a14031ccf9baed84c4",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0xfc8d513d1615c763865b984ea9c381032c14a983f80e5b2bd90b20b518329ed7",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0xfcc08928955d4e5e17e17e46d5adbb8011e0a8a74cabbdd3e138c367e89a4428",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0xfd3a8bacd3b2061cbe54f8d38cf13c5c87a92816937683652886dee936dfae10",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0xfdaf2549ea901a469b3e91cd1c4290fab376ef687547046751e10b7b461ff297",
                vec![196, 1, 128, 128, 128],
            ),
            (
                "0xfdbb8ddca8cecfe275da1ea1c36e494536f581d64ddf0c4f2e6dae9c7d891427",
                vec![
                    228, 1, 128, 160, 211, 217, 131, 159, 135, 194, 159, 176, 7, 253, 153, 40, 211,
                    139, 191, 132, 239, 8, 159, 12, 214, 64, 200, 56, 244, 164, 38, 49, 232, 40,
                    198, 103, 128,
                ],
            ),
            (
                "0xfe2149c5c256a5eb2578c013d33e3af6a87a514965c7ddf4a8131e2d978f09f9",
                vec![196, 128, 1, 128, 128],
            ),
            (
                "0xfe2511e8a33ac9973b773aaedcb4daa73ae82481fe5a1bf78b41281924260cf5",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
            (
                "0xfe6e594c507ec0ac14917f7a8032f83cd0c3c58b461d459b822190290852c0e1",
                vec![201, 128, 133, 23, 72, 118, 232, 0, 128, 128],
            ),
        ];

        // Create a store and load it up with the accounts
        let store = Store::new("null", EngineType::InMemory).unwrap();
        let mut state_trie = store.new_state_trie_for_test();
        for (address, account) in accounts {
            let hashed_address = H256::from_str(address).unwrap();
            let account = AccountState::from(AccountStateSlim::decode(&account).unwrap());
            state_trie
                .insert(hashed_address.encode_to_vec(), account.encode_to_vec())
                .unwrap();
        }
        (store, state_trie.hash().unwrap())
    }
}

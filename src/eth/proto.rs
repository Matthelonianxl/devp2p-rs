use rlp::{Encodable, Decodable, RlpStream, DecoderError, UntrustedRlp};
use bigint::{Address, Gas, H256, U256, B256};
use block::{Header, Transaction, Block};

/// ETH message version 62 and 63
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ETHMessage {
    Status {
        protocol_version: usize,
        network_id: usize,
        total_difficulty: U256,
        best_hash: H256,
        genesis_hash: H256,
    },
    NewBlockHashes(Vec<(H256, U256)>),
    Transactions(Vec<Transaction>),
    GetBlockHeadersByNumber {
        number: U256, // TODO: this can also be a hash.
        max_headers: usize,
        skip: usize,
        reverse: bool,
    },
    GetBlockHeadersByHash {
        hash: H256,
        max_headers: usize,
        skip: usize,
        reverse: bool,
    },
    BlockHeaders(Vec<Header>),
    GetBlockBodies(Vec<H256>),
    BlockBodies(Vec<(Vec<Transaction>, Vec<Header>)>),
    NewBlock {
        block: Block,
        total_difficulty: U256
    },
    Unknown,
}

impl ETHMessage {
    /// Get the message id of the ETH message
    pub fn id(&self) -> usize {
        match self {
            &ETHMessage::Status { .. } => 0,
            &ETHMessage::NewBlockHashes(_) => 1,
            &ETHMessage::Transactions(_) => 2,
            &ETHMessage::GetBlockHeadersByNumber { .. } => 3,
            &ETHMessage::GetBlockHeadersByHash { .. } => 3,
            &ETHMessage::BlockHeaders(_) => 4,
            &ETHMessage::GetBlockBodies(_) => 5,
            &ETHMessage::BlockBodies(_) => 6,
            &ETHMessage::NewBlock { .. } => 7,
            &ETHMessage::Unknown => 127,
        }
    }

    /// Decode a RLP into ETH message using the given message id
    pub fn decode(rlp: &UntrustedRlp, id: usize) -> Result<Self, DecoderError> {
        Ok(match id {
            0 => {
                ETHMessage::Status {
                    protocol_version: rlp.val_at(0)?,
                    network_id: rlp.val_at(1)?,
                    total_difficulty: rlp.val_at(2)?,
                    best_hash: rlp.val_at(3)?,
                    genesis_hash: rlp.val_at(4)?,
                }
            },
            1 => {
                let mut r = Vec::new();
                for i in 0..rlp.item_count()? {
                    let d = rlp.at(i)?;
                    r.push((d.val_at(0)?, d.val_at(1)?));
                }
                ETHMessage::NewBlockHashes(r)
            },
            2 => {
                ETHMessage::Transactions(rlp.as_list()?)
            },
            3 => {
                let reverse: u32 = rlp.val_at(3)?;
                if rlp.at(0)?.size() == 32 {
                    ETHMessage::GetBlockHeadersByHash {
                        hash: rlp.val_at(0)?,
                        max_headers: rlp.val_at(1)?,
                        skip: rlp.val_at(2)?,
                        reverse: if reverse == 0 { false } else { true },
                    }
                } else {
                    ETHMessage::GetBlockHeadersByNumber {
                        number: rlp.val_at(0)?,
                        max_headers: rlp.val_at(1)?,
                        skip: rlp.val_at(2)?,
                        reverse: if reverse == 0 { false } else { true },
                    }
                }
            },
            4 => {
                ETHMessage::BlockHeaders(rlp.as_list()?)
            },
            5 => {
                ETHMessage::GetBlockBodies(rlp.as_list()?)
            },
            6 => {
                let mut r = Vec::new();
                for i in 0..rlp.item_count()? {
                    let d = rlp.at(i)?;
                    r.push((d.list_at(0)?, d.list_at(1)?));
                }
                ETHMessage::BlockBodies(r)
            },
            7 => {
                ETHMessage::NewBlock {
                    block: rlp.val_at(0)?,
                    total_difficulty: rlp.val_at(1)?,
                }
            },
            _ => {
                ETHMessage::Unknown
            },
        })
    }
}

impl Encodable for ETHMessage {
    fn rlp_append(&self, s: &mut RlpStream) {
        match self {
            &ETHMessage::Status {
                protocol_version, network_id, total_difficulty, best_hash, genesis_hash
            } => {
                s.begin_list(5);
                s.append(&protocol_version);
                s.append(&network_id);
                s.append(&total_difficulty);
                s.append(&best_hash);
                s.append(&genesis_hash);
            },
            &ETHMessage::NewBlockHashes(ref hashes) => {
                s.begin_list(hashes.len());
                for &(hash, number) in hashes {
                    s.begin_list(2);
                    s.append(&hash);
                    s.append(&number);
                }
            },
            &ETHMessage::Transactions(ref transactions) => {
                s.append_list(&transactions);
            },
            &ETHMessage::GetBlockHeadersByNumber {
                number,
                max_headers, skip, reverse
            } => {
                s.begin_list(4);
                s.append(&number);
                s.append(&max_headers);
                s.append(&skip);
                s.append(&if reverse { 1u32 } else { 0u32 });
            },
            &ETHMessage::GetBlockHeadersByHash {
                hash,
                max_headers, skip, reverse
            } => {
                s.begin_list(4);
                s.append(&hash);
                s.append(&max_headers);
                s.append(&skip);
                s.append(&if reverse { 1u32 } else { 0u32 });
            },
            &ETHMessage::BlockHeaders(ref headers) => {
                s.append_list(&headers);
            },
            &ETHMessage::GetBlockBodies(ref hashes) => {
                s.append_list(&hashes);
            },
            &ETHMessage::BlockBodies(ref bodies) => {
                for &(ref transactions, ref ommers) in bodies {
                    s.begin_list(2);
                    s.append_list(&transactions);
                    s.append_list(&ommers);
                }
            },
            &ETHMessage::NewBlock { ref block, ref total_difficulty } => {
                s.begin_list(2);
                s.append(block);
                s.append(total_difficulty);
            }
            &ETHMessage::Unknown => {
                s.begin_list(0);
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ETHMessage;
    use rlp::{self, Encodable, Decodable, RlpStream, DecoderError, UntrustedRlp};
    use bigint::H256;

    #[test]
    fn test_new_block_hashes_message() {
        let data: [u8; 39] = [230, 229, 160, 11, 242, 248, 253, 140, 225, 253, 52, 9, 21, 69, 46, 23, 90, 133, 106, 179, 73, 226, 76, 239, 254, 249, 176, 45, 113, 180, 213, 192, 189, 117, 194, 131, 62, 213, 12];
        ETHMessage::decode(&UntrustedRlp::new(&data), 1).unwrap();
    }

    #[test]
    fn test_get_block_headers_message() {
        let data: [u8; 8] = [199, 131, 29, 76, 0, 1, 128, 128];
        ETHMessage::decode(&UntrustedRlp::new(&data), 3).unwrap();
    }

    #[test]
    fn test_get_block_headers_hash_message() {
        let hash = H256::random();
        let message = ETHMessage::GetBlockHeadersByHash {
            hash, max_headers: 2048, skip: 0, reverse: false,
        };
        assert_eq!(message, ETHMessage::decode(&UntrustedRlp::new(&rlp::encode(&message)), 3).unwrap());
    }

    #[test]
    fn test_block_headers_message() {
        let data: [u8; 2148] = [249, 8, 97, 249, 2, 20, 160, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 160, 29, 204, 77, 232, 222, 199, 93, 122, 171, 133, 181, 103, 182, 204, 212, 26, 211, 18, 69, 27, 148, 138, 116, 19, 240, 161, 66, 253, 64, 212, 147, 71, 148, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 160, 215, 248, 151, 79, 181, 172, 120, 217, 172, 9, 155, 154, 213, 1, 139, 237, 194, 206, 10, 114, 218, 209, 130, 122, 23, 9, 218, 48, 88, 15, 5, 68, 160, 86, 232, 31, 23, 27, 204, 85, 166, 255, 131, 69, 230, 146, 192, 248, 110, 91, 72, 224, 27, 153, 108, 173, 192, 1, 98, 47, 181, 227, 99, 180, 33, 160, 86, 232, 31, 23, 27, 204, 85, 166, 255, 131, 69, 230, 146, 192, 248, 110, 91, 72, 224, 27, 153, 108, 173, 192, 1, 98, 47, 181, 227, 99, 180, 33, 185, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 133, 4, 0, 0, 0, 0, 128, 130, 19, 136, 128, 128, 160, 17, 187, 232, 219, 78, 52, 123, 78, 140, 147, 124, 28, 131, 112, 228, 181, 237, 51, 173, 179, 219, 105, 203, 219, 122, 56, 225, 229, 11, 27, 130, 250, 160, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 136, 0, 0, 0, 0, 0, 0, 0, 66, 249, 2, 17, 160, 212, 229, 103, 64, 248, 118, 174, 248, 192, 16, 184, 106, 64, 213, 245, 103, 69, 161, 24, 208, 144, 106, 52, 230, 154, 236, 140, 13, 177, 203, 143, 163, 160, 29, 204, 77, 232, 222, 199, 93, 122, 171, 133, 181, 103, 182, 204, 212, 26, 211, 18, 69, 27, 148, 138, 116, 19, 240, 161, 66, 253, 64, 212, 147, 71, 148, 5, 165, 110, 45, 82, 200, 23, 22, 24, 131, 245, 12, 68, 28, 50, 40, 207, 229, 77, 159, 160, 214, 126, 77, 69, 3, 67, 4, 100, 37, 174, 66, 113, 71, 67, 83, 133, 122, 184, 96, 219, 192, 161, 221, 230, 75, 65, 181, 205, 58, 83, 43, 243, 160, 86, 232, 31, 23, 27, 204, 85, 166, 255, 131, 69, 230, 146, 192, 248, 110, 91, 72, 224, 27, 153, 108, 173, 192, 1, 98, 47, 181, 227, 99, 180, 33, 160, 86, 232, 31, 23, 27, 204, 85, 166, 255, 131, 69, 230, 146, 192, 248, 110, 91, 72, 224, 27, 153, 108, 173, 192, 1, 98, 47, 181, 227, 99, 180, 33, 185, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 133, 3, 255, 128, 0, 0, 1, 130, 19, 136, 128, 132, 85, 186, 66, 36, 153, 71, 101, 116, 104, 47, 118, 49, 46, 48, 46, 48, 47, 108, 105, 110, 117, 120, 47, 103, 111, 49, 46, 52, 46, 50, 160, 150, 155, 144, 13, 226, 123, 106, 198, 166, 119, 66, 54, 93, 214, 95, 85, 160, 82, 108, 65, 253, 24, 225, 177, 111, 26, 18, 21, 194, 230, 111, 89, 136, 83, 155, 212, 151, 159, 239, 30, 196, 249, 2, 24, 160, 136, 233, 109, 69, 55, 190, 164, 217, 192, 93, 18, 84, 153, 7, 179, 37, 97, 211, 191, 49, 244, 90, 174, 115, 76, 220, 17, 159, 19, 64, 108, 182, 160, 29, 204, 77, 232, 222, 199, 93, 122, 171, 133, 181, 103, 182, 204, 212, 26, 211, 18, 69, 27, 148, 138, 116, 19, 240, 161, 66, 253, 64, 212, 147, 71, 148, 221, 47, 30, 110, 73, 130, 2, 232, 109, 143, 84, 66, 175, 89, 101, 128, 164, 240, 60, 44, 160, 73, 67, 217, 65, 99, 116, 17, 16, 116, 148, 218, 158, 200, 188, 4, 53, 157, 115, 27, 253, 8, 183, 43, 77, 14, 220, 189, 76, 210, 236, 179, 65, 160, 86, 232, 31, 23, 27, 204, 85, 166, 255, 131, 69, 230, 146, 192, 248, 110, 91, 72, 224, 27, 153, 108, 173, 192, 1, 98, 47, 181, 227, 99, 180, 33, 160, 86, 232, 31, 23, 27, 204, 85, 166, 255, 131, 69, 230, 146, 192, 248, 110, 91, 72, 224, 27, 153, 108, 173, 192, 1, 98, 47, 181, 227, 99, 180, 33, 185, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 133, 3, 255, 0, 16, 0, 2, 130, 19, 136, 128, 132, 85, 186, 66, 65, 160, 71, 101, 116, 104, 47, 118, 49, 46, 48, 46, 48, 45, 48, 99, 100, 99, 55, 54, 52, 55, 47, 108, 105, 110, 117, 120, 47, 103, 111, 49, 46, 52, 160, 47, 7, 144, 197, 170, 49, 171, 148, 25, 94, 31, 100, 67, 214, 69, 175, 91, 117, 196, 108, 4, 251, 249, 145, 23, 17, 25, 138, 12, 232, 253, 218, 136, 184, 83, 250, 38, 26, 134, 170, 158, 249, 2, 24, 160, 180, 149, 161, 215, 230, 102, 49, 82, 174, 146, 112, 141, 164, 132, 51, 55, 185, 88, 20, 96, 21, 162, 128, 47, 65, 147, 164, 16, 4, 70, 152, 201, 160, 107, 23, 185, 56, 198, 228, 239, 24, 178, 106, 216, 27, 156, 163, 81, 95, 39, 253, 156, 78, 130, 170, 197, 106, 31, 216, 234, 178, 136, 120, 94, 65, 148, 80, 136, 214, 35, 186, 15, 207, 1, 49, 224, 137, 122, 145, 115, 74, 77, 131, 89, 106, 160, 160, 118, 171, 11, 137, 158, 131, 135, 67, 111, 242, 101, 142, 41, 136, 248, 60, 191, 26, 241, 89, 11, 159, 233, 254, 202, 55, 20, 248, 209, 130, 73, 64, 160, 86, 232, 31, 23, 27, 204, 85, 166, 255, 131, 69, 230, 146, 192, 248, 110, 91, 72, 224, 27, 153, 108, 173, 192, 1, 98, 47, 181, 227, 99, 180, 33, 160, 86, 232, 31, 23, 27, 204, 85, 166, 255, 131, 69, 230, 146, 192, 248, 110, 91, 72, 224, 27, 153, 108, 173, 192, 1, 98, 47, 181, 227, 99, 180, 33, 185, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 133, 3, 254, 128, 47, 254, 3, 130, 19, 136, 128, 132, 85, 186, 66, 96, 160, 71, 101, 116, 104, 47, 118, 49, 46, 48, 46, 48, 45, 102, 99, 55, 57, 100, 51, 50, 100, 47, 108, 105, 110, 117, 120, 47, 103, 111, 49, 46, 52, 160, 101, 225, 46, 236, 35, 254, 101, 85, 230, 188, 219, 71, 170, 37, 38, 154, 225, 6, 229, 241, 107, 84, 225, 233, 45, 206, 226, 94, 28, 138, 208, 55, 136, 46, 147, 68, 224, 203, 222, 131, 206];
        ETHMessage::decode(&UntrustedRlp::new(&data), 4).unwrap();
    }
}

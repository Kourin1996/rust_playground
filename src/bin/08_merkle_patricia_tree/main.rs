use sha3::{Digest, Keccak256};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Nibbles(Vec<u8>);

impl Nibbles {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut nibbles = Vec::with_capacity(bytes.len() * 2);
        for &byte in bytes {
            nibbles.push(byte >> 4);
            nibbles.push(byte & 0x0f);
        }
        Nibbles(nibbles)
    }

    pub fn slice_from(&self, start: usize) -> Nibbles {
        Nibbles(self.0[start..].to_vec())
    }

    pub fn slice(&self, start: usize, end: usize) -> Nibbles {
        Nibbles(self.0[start..end].to_vec())
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn at(&self, index: usize) -> u8 {
        self.0[index]
    }

    pub fn common_prefix_length(&self, other: &Nibbles) -> usize {
        self.0
            .iter()
            .zip(other.0.iter())
            .take_while(|(a, b)| a == b)
            .count()
    }

    pub fn hex_prefix_encode(&self, is_leaf: bool) -> Vec<u8> {
        let odd = self.len() % 2 == 1;
        let prefix: u8 = match (is_leaf, odd) {
            (true, true) => 0b11,   // leaf & odd -> 3
            (true, false) => 0b10,  // leaf & even -> 2
            (false, true) => 0b01,  // extension & odd -> 1
            (false, false) => 0b00, // extension & even -> 0
        };

        let mut result = Vec::with_capacity(1 + self.len() / 2);

        // if the size is odd, the first byte will be flag & the first nibblel
        // if the size is even, the first byte will be flag & 4 bits zero
        if odd {
            result.push(prefix << 4 | self.0[0]);
        } else {
            result.push(prefix << 4);
        }

        let remainings = if odd { &self.0[1..] } else { &self.0[0..] };
        for chunk in remainings.chunks(2) {
            result.push(chunk[0] << 4 | chunk[1]);
        }

        result
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    Empty,

    Leaf {
        partial: Nibbles,
        value: Vec<u8>,
    },

    Extension {
        partial: Nibbles,
        child: Box<Node>,
    },

    Branch {
        children: Box<[Node; 16]>,
        value: Option<Vec<u8>>,
    },
}

impl Default for Node {
    fn default() -> Self {
        Node::Empty
    }
}

impl Node {
    fn new_branch() -> Self {
        Node::Branch {
            children: Box::new(std::array::from_fn(|_| Node::Empty)),
            value: None,
        }
    }

    pub fn rlp_encode(&self) -> Vec<u8> {
        match self {
            Node::Empty => {
                vec![0x80]
            }
            Node::Leaf { partial, value } => {
                let encoded_path = partial.hex_prefix_encode(true);
                rlp_encode_list(&[rlp_encode_bytes(&encoded_path), rlp_encode_bytes(value)])
            }

            Node::Extension { partial, child } => {
                let encoded_path = partial.hex_prefix_encode(false);
                rlp_encode_list(&[rlp_encode_bytes(&encoded_path), node_reference(child)])
            }

            Node::Branch { children, value } => {
                let mut items: Vec<Vec<u8>> =
                    children.iter().map(|child| node_reference(child)).collect();
                match value {
                    Some(v) => items.push(rlp_encode_bytes(v)),
                    None => items.push(vec![0x80]),
                }
                rlp_encode_list(&items)
            }
        }
    }

    pub fn hash(&self) -> Vec<u8> {
        keccak256(&self.rlp_encode())
    }
}

pub struct Trie {
    root: Node,
}

impl Trie {
    pub fn new() -> Self {
        Trie { root: Node::Empty }
    }

    pub fn get(&self, key: &[u8]) -> Option<&[u8]> {
        let nibbles = Nibbles::from_bytes(key);
        Self::get_node(&self.root, &nibbles, 0)
    }

    pub fn insert(&mut self, key: &[u8], value: Vec<u8>) {
        let nibbles = Nibbles::from_bytes(key);
        let root = std::mem::take(&mut self.root);
        self.root = Self::insert_node(root, &nibbles, 0, value);
    }

    pub fn root_hash(&self) -> Vec<u8> {
        self.root.hash()
    }

    pub fn root_hash_hex(&self) -> String {
        hex::encode(self.root_hash())
    }

    fn get_node<'a>(node: &'a Node, key: &Nibbles, pos: usize) -> Option<&'a [u8]> {
        match node {
            Node::Empty => None,

            Node::Leaf { partial, value } => {
                let remaining = key.slice_from(pos);
                if remaining == *partial {
                    Some(value)
                } else {
                    None
                }
            }

            Node::Extension { partial, child } => {
                let remaining = key.slice_from(pos);
                if remaining.len() >= partial.len() && remaining.slice(0, partial.len()) == *partial
                {
                    Self::get_node(child, key, pos + partial.len())
                } else {
                    None
                }
            }

            Node::Branch { children, value } => {
                if pos == key.len() {
                    value.as_deref()
                } else {
                    Self::get_node(&children[key.at(pos) as usize], key, pos + 1)
                }
            }
        }
    }

    fn insert_node(node: Node, key: &Nibbles, pos: usize, value: Vec<u8>) -> Node {
        match node {
            Node::Empty => Node::Leaf {
                partial: key.slice_from(pos),
                value,
            },

            Node::Leaf {
                partial: existing_partial,
                value: existing_value,
            } => {
                let remaining = key.slice_from(pos);
                let common_len = existing_partial.common_prefix_length(&remaining);

                if common_len == existing_partial.len() && common_len == remaining.len() {
                    return Node::Leaf {
                        partial: existing_partial,
                        value,
                    };
                }

                let mut branch = Node::new_branch();

                if common_len == existing_partial.len() {
                    if let Node::Branch {
                        value: ref mut v, ..
                    } = branch
                    {
                        *v = Some(existing_value);
                    }
                } else {
                    let idx = existing_partial.at(common_len) as usize;
                    let child = Node::Leaf {
                        partial: existing_partial.slice_from(common_len + 1),
                        value: existing_value,
                    };
                    if let Node::Branch {
                        ref mut children, ..
                    } = branch
                    {
                        children[idx] = child;
                    }
                }

                if common_len == remaining.len() {
                    if let Node::Branch {
                        value: ref mut v, ..
                    } = branch
                    {
                        *v = Some(value);
                    }
                } else {
                    let idx = remaining.at(common_len) as usize;
                    let child = Node::Leaf {
                        partial: remaining.slice_from(common_len + 1),
                        value,
                    };
                    if let Node::Branch {
                        ref mut children, ..
                    } = branch
                    {
                        children[idx] = child;
                    }
                }

                if common_len > 0 {
                    Node::Extension {
                        partial: existing_partial.slice(0, common_len),
                        child: Box::new(branch),
                    }
                } else {
                    branch
                }
            }
            Node::Extension {
                partial: ext_partial,
                child: ext_child,
            } => {
                let remaining = key.slice_from(pos);
                let common_len = ext_partial.common_prefix_length(&remaining);

                if common_len == ext_partial.len() {
                    let new_child = Self::insert_node(*ext_child, key, pos + common_len, value);
                    return Node::Extension {
                        partial: ext_partial,
                        child: Box::new(new_child),
                    };
                }

                let mut branch = Node::new_branch();
                if common_len + 1 == ext_partial.len() {
                    let idx = ext_partial.at(common_len) as usize;
                    if let Node::Branch {
                        ref mut children, ..
                    } = branch
                    {
                        children[idx] = *ext_child;
                    }
                } else {
                    let idx = ext_partial.at(common_len) as usize;
                    let shortened = Node::Extension {
                        partial: ext_partial.slice_from(common_len + 1),
                        child: ext_child,
                    };

                    if let Node::Branch {
                        ref mut children, ..
                    } = branch
                    {
                        children[idx] = shortened;
                    }
                }

                if common_len == remaining.len() {
                    if let Node::Branch {
                        value: ref mut v, ..
                    } = branch
                    {
                        *v = Some(value);
                    }
                } else {
                    let idx = remaining.at(common_len) as usize;
                    let new_leaf = Node::Leaf {
                        partial: remaining.slice_from(common_len + 1),
                        value,
                    };
                    if let Node::Branch {
                        ref mut children, ..
                    } = branch
                    {
                        children[idx] = new_leaf;
                    }
                }

                if common_len > 0 {
                    Node::Extension {
                        partial: ext_partial.slice(0, common_len),
                        child: Box::new(branch),
                    }
                } else {
                    branch
                }
            }

            Node::Branch {
                mut children,
                value: branch_value,
            } => {
                if pos == key.len() {
                    Node::Branch {
                        children,
                        value: Some(value),
                    }
                } else {
                    let idx = key.at(pos) as usize;
                    let child = std::mem::take(&mut children[idx]);
                    children[idx] = Self::insert_node(child, key, pos + 1, value);
                    Node::Branch {
                        children,
                        value: branch_value,
                    }
                }
            }
        }
    }

    pub fn print_structure(&self) {
        println!("=== Trie Structure ===");
        Self::print_node(&self.root, 0);
        println!("=== Root Hash: {} ===", self.root_hash_hex());
    }

    fn print_node(node: &Node, depth: usize) {
        let indent = "  ".repeat(depth);
        match node {
            Node::Empty => {
                println!("{indent}(empty)");
            }
            Node::Leaf { partial, value } => {
                println!(
                    "{indent}Leaf {{ path: {:?}, value: \"{}\" }}",
                    partial.0,
                    String::from_utf8_lossy(value)
                );
            }
            Node::Extension { partial, child } => {
                println!("{indent}Extension {{ path: {:?} }}", partial.0);
                Self::print_node(child, depth + 1);
            }
            Node::Branch { children, value } => {
                if let Some(v) = value {
                    println!(
                        "{indent}Branch {{ value: \"{}\" }}",
                        String::from_utf8_lossy(v)
                    );
                } else {
                    println!("{indent}Branch");
                }
                for (i, child) in children.iter().enumerate() {
                    if *child != Node::Empty {
                        println!("{indent}  [{:x}]:", i);
                        Self::print_node(child, depth + 2);
                    }
                }
            }
        }
    }
}

fn encode_length(len: usize) -> Vec<u8> {
    if len == 0 {
        return vec![0];
    }

    let bytes = len.to_be_bytes();
    let first_non_zero = (len.leading_zeros() / 8) as usize;
    bytes[first_non_zero..].to_vec()
}

fn rlp_encode_bytes(data: &[u8]) -> Vec<u8> {
    if data.len() == 1 && data[0] < 0x80 {
        vec![data[0]]
    } else if data.len() < 56 {
        let mut result = vec![0x80 + data.len() as u8];
        result.extend_from_slice(data);
        result
    } else {
        let len_bytes = encode_length(data.len());
        let mut result = vec![0xb7 + len_bytes.len() as u8];
        result.extend_from_slice(&len_bytes);
        result.extend_from_slice(data);
        result
    }
}

fn rlp_encode_list(items: &[Vec<u8>]) -> Vec<u8> {
    let payload: Vec<u8> = items.iter().flat_map(|i| i.iter().copied()).collect();
    if payload.len() < 56 {
        let mut result = vec![0xc0 + payload.len() as u8];
        result.extend_from_slice(&payload);
        result
    } else {
        let len_bytes = encode_length(payload.len());
        let mut result = vec![0xF7 + len_bytes.len() as u8];
        result.extend_from_slice(&len_bytes);
        result.extend_from_slice(&payload);
        result
    }
}

fn node_reference(node: &Node) -> Vec<u8> {
    match node {
        Node::Empty => vec![0x80],
        _ => {
            let encoded = node.rlp_encode();
            if encoded.len() >= 32 {
                rlp_encode_bytes(&keccak256(&encoded))
            } else {
                encoded
            }
        }
    }
}

fn keccak256(data: &[u8]) -> Vec<u8> {
    let mut hasher = Keccak256::new();
    hasher.update(data);
    hasher.finalize().to_vec()
}

fn main() {
    demo_basic_operations();

    demo_trie_structure();

    demo_tamper_detection();
}

fn demo_basic_operations() {
    println!("━━━ Demo 1: Basic Insert / Get ━━━");
    println!();

    let mut trie = Trie::new();

    trie.insert(b"do", b"verb".to_vec());
    trie.insert(b"dog", b"puppy".to_vec());
    trie.insert(b"doge", b"coin".to_vec());
    trie.insert(b"horse", b"stallion".to_vec());

    assert_eq!(trie.get(b"do"), Some(b"verb".as_slice()));
    assert_eq!(trie.get(b"dog"), Some(b"puppy".as_slice()));
    assert_eq!(trie.get(b"doge"), Some(b"coin".as_slice()));
    assert_eq!(trie.get(b"horse"), Some(b"stallion".as_slice()));
    assert_eq!(trie.get(b"cat"), None);

    println!(
        "  get(\"do\")    = {:?}",
        trie.get(b"do").map(|v| std::str::from_utf8(v).unwrap())
    );
    println!(
        "  get(\"dog\")   = {:?}",
        trie.get(b"dog").map(|v| std::str::from_utf8(v).unwrap())
    );
    println!(
        "  get(\"doge\")  = {:?}",
        trie.get(b"doge").map(|v| std::str::from_utf8(v).unwrap())
    );
    println!(
        "  get(\"horse\") = {:?}",
        trie.get(b"horse").map(|v| std::str::from_utf8(v).unwrap())
    );
    println!("  get(\"cat\")   = {:?}", trie.get(b"cat"));
    println!();
    println!("  Root hash: 0x{}", trie.root_hash_hex());
    println!();
}

fn demo_trie_structure() {
    println!("━━━ Demo 2: Visualize Trie Structure ━━━");
    println!();
    println!("  key: \"do\"→\"verb\", \"dog\"→\"puppy\", \"doge\"→\"coin\"");
    println!();

    let mut trie = Trie::new();
    trie.insert(b"do", b"verb".to_vec());
    trie.insert(b"dog", b"puppy".to_vec());
    trie.insert(b"doge", b"coin".to_vec());
    trie.print_structure();
    println!();
}

fn demo_tamper_detection() {
    println!("━━━ Demo 3: Check the change of root hash ━━━");
    println!();

    let mut trie1 = Trie::new();
    trie1.insert(b"balance", b"100".to_vec());
    trie1.insert(b"nonce", b"5".to_vec());
    let hash1 = trie1.root_hash_hex();

    let mut trie2 = Trie::new();
    trie2.insert(b"balance", b"100".to_vec());
    trie2.insert(b"nonce", b"5".to_vec());
    let hash2 = trie2.root_hash_hex();

    let mut trie3 = Trie::new();
    trie3.insert(b"balance", b"101".to_vec()); // ← 100 → 101
    trie3.insert(b"nonce", b"5".to_vec());
    let hash3 = trie3.root_hash_hex();

    println!("  Original    : 0x{}", hash1);
    println!("  Same data   : 0x{}", hash2);
    println!("  Tampered(+1): 0x{}", hash3);
    println!();
    println!("  Same data match   : {}", hash1 == hash2);
    println!("  Tampered mismatch : {}", hash1 != hash3);
    assert_eq!(hash1, hash2);
    assert_ne!(hash1, hash3);
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nibbles_from_bytes() {
        assert_eq!(Nibbles::from_bytes(&[]), Nibbles(vec![]));
        assert_eq!(
            Nibbles::from_bytes(&[0x01, 0x02]),
            Nibbles(vec![0x0, 0x01, 0x0, 0x02])
        );
        assert_eq!(
            Nibbles::from_bytes(&[0xA1, 0xB2]),
            Nibbles(vec![0x0A, 0x01, 0x0B, 0x02])
        );
    }

    #[test]
    fn nibbles_slice() {
        let nibbles = Nibbles::from_bytes(&[0x12, 0x34, 0x56]);
        assert_eq!(
            nibbles.slice_from(0),
            Nibbles(vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06])
        );
        assert_eq!(nibbles.slice_from(2), Nibbles(vec![0x03, 0x04, 0x05, 0x06]));
        assert_eq!(nibbles.slice_from(6), Nibbles(vec![]));
    }

    #[test]
    fn nibbles_common_prefix_length() {
        assert_eq!(
            Nibbles(vec![0x01, 0x02, 0x03])
                .common_prefix_length(&Nibbles(vec![0x01, 0x02, 0x0A, 0x0B])),
            2
        );
        assert_eq!(
            Nibbles(vec![0x01, 0x02, 0x03, 0x04]).common_prefix_length(&Nibbles(vec![0x01, 0x02])),
            2
        );

        assert_eq!(
            Nibbles(vec![0x01, 0x02, 0x03, 0x04])
                .common_prefix_length(&Nibbles(vec![0x0A, 0x0B, 0x0C, 0x0D])),
            0
        );
    }

    #[test]
    fn test_hex_prefix_encode() {
        // extension, empty
        let nibbles = Nibbles(vec![]);
        assert_eq!(nibbles.hex_prefix_encode(false), vec![0x00]);

        // leaf, empty
        let nibbles = Nibbles(vec![]);
        assert_eq!(nibbles.hex_prefix_encode(true), vec![0x20]);

        // extension, odd
        let nibbles = Nibbles(vec![1, 2, 3]);
        assert_eq!(nibbles.hex_prefix_encode(false), vec![0x11, 0x23]);

        // leaf, odd
        let nibbles = Nibbles(vec![1, 2, 3]);
        assert_eq!(nibbles.hex_prefix_encode(true), vec![0x31, 0x23]);

        // extension, even
        let nibbles = Nibbles(vec![0, 1, 2, 3]);
        assert_eq!(nibbles.hex_prefix_encode(false), vec![0x00, 0x01, 0x23]);

        // leaf, even
        let nibbles = Nibbles(vec![0, 1, 2, 3]);
        assert_eq!(nibbles.hex_prefix_encode(true), vec![0x20, 0x01, 0x23]);

        // 1 nibble only: extension
        let nibbles = Nibbles(vec![0x0f]);
        assert_eq!(nibbles.hex_prefix_encode(false), vec![0x1f]);

        // 1 nibble only: leaf
        let nibbles = Nibbles(vec![0x0f]);
        assert_eq!(nibbles.hex_prefix_encode(true), vec![0x3f]);

        // 2 nibbles only: extension
        let nibbles = Nibbles(vec![0x0a, 0x0b]);
        assert_eq!(nibbles.hex_prefix_encode(false), vec![0x00, 0xab]);

        // 2 nibbles only: leaf
        let nibbles = Nibbles(vec![0x0a, 0x0b]);
        assert_eq!(nibbles.hex_prefix_encode(true), vec![0x20, 0xab]);
    }

    #[test]
    fn test_node_rlp_encode() {
        // Empty
        let node = Node::Empty;
        assert_eq!(node.rlp_encode(), vec![0x80]);

        // Leaf
        let node = Node::Leaf {
            partial: Nibbles(vec![1, 2, 3]),
            value: b"cat".to_vec(),
        };
        assert_eq!(
            node.rlp_encode(),
            vec![0xC7, 0x82, 0x31, 0x23, 0x83, 0x63, 0x61, 0x74]
        );

        // Extension
        let node = Node::Extension {
            partial: Nibbles(vec![0x0a, 0x0b]),
            child: Box::new(Node::Leaf {
                partial: Nibbles(vec![0x0f]),
                value: b"z".to_vec(),
            }),
        };
        assert_eq!(
            node.rlp_encode(),
            vec![0xC6, 0x82, 0x00, 0xab, 0xC2, 0x3f, 0x7a]
        );

        // Branch
        let mut children = Box::new(std::array::from_fn(|_| Node::Empty));
        children[6] = Node::Leaf {
            partial: Nibbles(vec![7]),
            value: b"p".to_vec(),
        };

        let node = Node::Branch {
            children,
            value: Some(b"v".to_vec()),
        };

        let mut expected = vec![0xD3];
        expected.extend_from_slice(&[0x80; 6]);
        expected.extend_from_slice(&[0xC2, 0x37, 0x70]);
        expected.extend_from_slice(&[0x80; 9]);
        expected.push(0x76);

        assert_eq!(node.rlp_encode(), expected);
    }

    #[test]
    fn test_trie_get() {
        // 1. Empty Trie
        let trie = Trie::new();
        assert_eq!(trie.get(b"missing"), None);

        // 2. Trie holding single value
        let mut trie = Trie::new();
        trie.insert(b"key", b"value".to_vec());
        assert_eq!(trie.get(b"key"), Some(b"value".as_slice()));
        assert_eq!(trie.get(b"other"), None);

        // 3. Trie holding multiple values
        let mut trie = Trie::new();
        trie.insert(b"do", b"verb".to_vec());
        trie.insert(b"dog", b"puppy".to_vec());
        trie.insert(b"doge", b"coin".to_vec());
        trie.insert(b"horse", b"stallion".to_vec());

        assert_eq!(trie.get(b"do"), Some(b"verb".as_slice()));
        assert_eq!(trie.get(b"dog"), Some(b"puppy".as_slice()));
        assert_eq!(trie.get(b"doge"), Some(b"coin".as_slice()));
        assert_eq!(trie.get(b"horse"), Some(b"stallion".as_slice()));

        // 4. Not found
        assert_eq!(trie.get(b"d"), None);
        assert_eq!(trie.get(b"dogecoin"), None);
        assert_eq!(trie.get(b"hors"), None);
        assert_eq!(trie.get(b"cat"), None);

        // 5. Trie with the value whose key share a common prefix
        let mut trie = Trie::new();
        trie.insert(b"ab", b"prefix".to_vec());
        trie.insert(b"abc", b"first".to_vec());
        trie.insert(b"abd", b"second".to_vec());

        assert_eq!(trie.get(b"ab"), Some(b"prefix".as_slice()));
        assert_eq!(trie.get(b"abc"), Some(b"first".as_slice()));
        assert_eq!(trie.get(b"abd"), Some(b"second".as_slice()));
        assert_eq!(trie.get(b"a"), None);
        assert_eq!(trie.get(b"abe"), None);

        // 6. Overwrite
        let mut trie = Trie::new();
        trie.insert(b"key", b"old".to_vec());
        assert_eq!(trie.get(b"key"), Some(b"old".as_slice()));

        trie.insert(b"key", b"new".to_vec());
        assert_eq!(trie.get(b"key"), Some(b"new".as_slice()));
    }

    #[test]
    fn it_works() {
        assert_eq!(true, 1 == 1);
    }
}

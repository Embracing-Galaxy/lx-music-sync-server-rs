use openssl::hash::{hash, MessageDigest};
use rand::Rng;

#[test]
fn test_md5() {
    let mut rng = rand::rng();
    let data: Vec<u8> = (0..32).map(|_| rng.random()).collect();
    let res = hash(MessageDigest::md5(), &data).unwrap();
    assert_eq!(res.len(), 16);
}

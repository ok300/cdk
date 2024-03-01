//! Diffie-Hellmann key exchange

use bitcoin::hashes::{sha256, Hash};
#[cfg(feature = "mint")]
pub use mint::{sign_message, verify_message};
#[cfg(feature = "wallet")]
pub use wallet::{blind_message, construct_proofs, unblind_message};

use crate::error::Error;

const DOMAIN_SEPARATOR: &[u8; 28] = b"Secp256k1_HashToCurve_Cashu_";

pub fn hash_to_curve(message: &[u8]) -> Result<k256::PublicKey, Error> {
    let msg_to_hash = [DOMAIN_SEPARATOR, message].concat();

    let msg_hash = sha256::Hash::hash(&msg_to_hash).to_byte_array();

    let mut counter = 0;
    while counter < 2_u32.pow(16) {
        let mut bytes_to_hash = Vec::with_capacity(36);
        bytes_to_hash.extend_from_slice(&msg_hash);
        bytes_to_hash.extend_from_slice(&counter.to_le_bytes());

        let hash = sha256::Hash::hash(&bytes_to_hash);
        match k256::PublicKey::from_sec1_bytes(
            &[0x02u8]
                .iter()
                .chain(&hash.to_byte_array())
                .cloned()
                .collect::<Vec<u8>>(),
        ) {
            Ok(pubkey) => return Ok(pubkey),
            Err(_) => {
                counter += 1;
            }
        }
    }

    Err(Error::NoValidPoint)
}

#[cfg(feature = "wallet")]
mod wallet {
    use std::ops::Mul;

    use k256::{ProjectivePoint, Scalar, SecretKey};

    use super::hash_to_curve;
    use crate::error;
    use crate::nuts::{BlindedSignature, Keys, Proof, Proofs, PublicKey, *};
    use crate::secret::Secret;

    /// Blind Message Alice Step one
    pub fn blind_message(
        secret: &[u8],
        blinding_factor: Option<SecretKey>,
    ) -> Result<(PublicKey, SecretKey), error::wallet::Error> {
        let y = hash_to_curve(secret)?;

        let r: SecretKey = match blinding_factor {
            Some(sec_key) => sec_key,
            None => SecretKey::random(&mut rand::thread_rng()),
        };

        let b = ProjectivePoint::from(y) + ProjectivePoint::from(&r.public_key());

        Ok((k256::PublicKey::try_from(b)?.into(), r))
    }

    /// Unblind Message (Alice Step 3)
    pub fn unblind_message(
        // C_
        blinded_key: PublicKey,
        r: SecretKey,
        // A
        mint_pubkey: PublicKey,
    ) -> Result<PublicKey, error::wallet::Error> {
        // C
        // Unblinded message
        let c = ProjectivePoint::from(Into::<k256::PublicKey>::into(blinded_key).as_affine())
            - Into::<k256::PublicKey>::into(mint_pubkey)
                .as_affine()
                .mul(Scalar::from(r.as_scalar_primitive()));

        Ok(k256::PublicKey::try_from(c)?.into())
    }

    /// Construct Proof
    pub fn construct_proofs(
        promises: Vec<BlindedSignature>,
        rs: Vec<nut01::SecretKey>,
        secrets: Vec<Secret>,
        keys: &Keys,
    ) -> Result<Proofs, error::wallet::Error> {
        let mut proofs = vec![];
        for (i, promise) in promises.into_iter().enumerate() {
            let blinded_c = promise.c;
            let a: PublicKey = keys
                .amount_key(promise.amount)
                .ok_or(error::wallet::Error::CustomError(
                    "Could not get proofs".to_string(),
                ))?
                .to_owned();

            let unblinded_signature = unblind_message(blinded_c, rs[i].clone().into(), a)?;

            let proof = Proof::new(
                promise.amount,
                promise.keyset_id,
                secrets[i].clone(),
                unblinded_signature,
            );

            proofs.push(proof);
        }

        Ok(proofs)
    }
}

#[cfg(feature = "mint")]
mod mint {
    use std::fmt::Debug;
    use std::ops::Mul;

    use k256::{Scalar, SecretKey};

    use super::hash_to_curve;
    use crate::error;

    /// Sign Blinded Message (Step2 bob)
    pub fn sign_message(
        a: SecretKey,
        blinded_message: k256::PublicKey,
    ) -> Result<k256::PublicKey, error::mint::Error> {
        Ok(k256::PublicKey::try_from(
            blinded_message
                .as_affine()
                .mul(Scalar::from(a.as_scalar_primitive())),
        )?)
    }

    /// Verify Message
    pub fn verify_message<V>(
        a: SecretKey,
        unblinded_message: k256::PublicKey,
        msg: V,
    ) -> Result<(), error::mint::Error>
    where
        V: TryInto<Vec<u8>>,
        <V as TryInto<Vec<u8>>>::Error: Debug,
        error::mint::Error: From<<V as TryInto<Vec<u8>>>::Error>,
    {
        // Y
        let y = hash_to_curve(&msg.try_into()?)?;

        if unblinded_message
            == k256::PublicKey::try_from(*y.as_affine() * Scalar::from(a.as_scalar_primitive()))?
        {
            return Ok(());
        }

        Err(error::mint::Error::TokenNotVerifed)
    }
}

#[cfg(test)]
mod tests {
    use hex::decode;
    use k256::elliptic_curve::scalar::ScalarPrimitive;

    use super::*;

    #[cfg(feature = "wallet")]
    mod wallet_tests {
        use k256::SecretKey;

        use super::*;
        use crate::nuts::PublicKey;

        #[test]
        fn test_hash_to_curve() {
            let secret = "0000000000000000000000000000000000000000000000000000000000000000";
            let sec_hex = decode(secret).unwrap();

            let y = hash_to_curve(&sec_hex).unwrap();
            let expected_y = k256::PublicKey::from_sec1_bytes(
                &hex::decode("024cce997d3b518f739663b757deaec95bcd9473c30a14ac2fd04023a739d1a725")
                    .unwrap(),
            )
            .unwrap();
            println!("{}", hex::encode(y.to_sec1_bytes()));
            assert_eq!(y, expected_y);

            let secret = "0000000000000000000000000000000000000000000000000000000000000001";
            let sec_hex = decode(secret).unwrap();
            let y = hash_to_curve(&sec_hex).unwrap();
            let expected_y = k256::PublicKey::from_sec1_bytes(
                &hex::decode("022e7158e11c9506f1aa4248bf531298daa7febd6194f003edcd9b93ade6253acf")
                    .unwrap(),
            )
            .unwrap();
            println!("{}", hex::encode(y.to_sec1_bytes()));
            assert_eq!(y, expected_y);
            // Note that this message will take a few iterations of the loop before finding
            // a valid point
            let secret = "0000000000000000000000000000000000000000000000000000000000000002";
            let sec_hex = decode(secret).unwrap();
            let y = hash_to_curve(&sec_hex).unwrap();
            let expected_y = k256::PublicKey::from_sec1_bytes(
                &hex::decode("026cdbe15362df59cd1dd3c9c11de8aedac2106eca69236ecd9fbe117af897be4f")
                    .unwrap(),
            )
            .unwrap();
            println!("{}", hex::encode(y.to_sec1_bytes()));
            assert_eq!(y, expected_y);
        }

        #[test]
        fn test_blind_message() {
            let message = "d341ee4871f1f889041e63cf0d3823c713eea6aff01e80f1719f08f9e5be98f6";
            let sec: crate::nuts::SecretKey = crate::nuts::SecretKey::from_hex(
                "99fce58439fc37412ab3468b73db0569322588f62fb3a49182d67e23d877824a",
            )
            .unwrap();

            let (b, r) =
                blind_message(&hex::decode(message).unwrap(), Some(sec.clone().into())).unwrap();

            assert_eq!(sec, r.into());

            assert_eq!(
                b.to_string(),
                PublicKey::from(
                    k256::PublicKey::from_sec1_bytes(
                        &hex::decode(
                            "033b1a9737a40cc3fd9b6af4b723632b76a67a36782596304612a6c2bfb5197e6d"
                        )
                        .unwrap()
                    )
                    .unwrap()
                )
                .to_string()
            );

            let message = "f1aaf16c2239746f369572c0784d9dd3d032d952c2d992175873fb58fae31a60";
            let sec: crate::nuts::SecretKey = crate::nuts::SecretKey::from_hex(
                "f78476ea7cc9ade20f9e05e58a804cf19533f03ea805ece5fee88c8e2874ba50",
            )
            .unwrap();

            let (b, r) =
                blind_message(&hex::decode(message).unwrap(), Some(sec.clone().into())).unwrap();

            assert_eq!(sec, r.into());

            assert_eq!(
                b.to_string(),
                PublicKey::from(
                    k256::PublicKey::from_sec1_bytes(
                        &hex::decode(
                            "029bdf2d716ee366eddf599ba252786c1033f47e230248a4612a5670ab931f1763"
                        )
                        .unwrap()
                    )
                    .unwrap()
                )
                .to_string()
            );
        }

        #[test]
        fn test_unblind_message() {
            let blinded_key = k256::PublicKey::from_sec1_bytes(
                &hex::decode("02a9acc1e48c25eeeb9289b5031cc57da9fe72f3fe2861d264bdc074209b107ba2")
                    .unwrap(),
            )
            .unwrap();

            let r = SecretKey::new(ScalarPrimitive::ONE);
            let a = k256::PublicKey::from_sec1_bytes(
                &hex::decode("020000000000000000000000000000000000000000000000000000000000000001")
                    .unwrap(),
            )
            .unwrap();

            let unblinded = unblind_message(blinded_key.into(), r, a.into()).unwrap();

            assert_eq!(
                Into::<PublicKey>::into(
                    k256::PublicKey::from_sec1_bytes(
                        &hex::decode(
                            "03c724d7e6a5443b39ac8acf11f40420adc4f99a02e7cc1b57703d9391f6d129cd"
                        )
                        .unwrap()
                    )
                    .unwrap()
                ),
                unblinded
            );
        }
    }

    #[cfg(feature = "mint")]
    mod mint_test {

        use k256::SecretKey;

        use super::{hash_to_curve, *};
        use crate::secret::Secret;

        #[test]
        fn test_sign_message() {
            use super::*;
            let message = "test_message";
            let sec = SecretKey::new(ScalarPrimitive::ONE);

            let (blinded_message, _r) = blind_message(message.as_bytes(), Some(sec)).unwrap();

            // A
            let bob_sec = SecretKey::new(ScalarPrimitive::ONE);

            // C_
            let signed = sign_message(bob_sec, blinded_message.clone().into()).unwrap();

            assert_eq!(
                signed,
                k256::PublicKey::from_sec1_bytes(
                    &hex::decode(
                        "025cc16fe33b953e2ace39653efb3e7a7049711ae1d8a2f7a9108753f1cdea742b"
                    )
                    .unwrap()
                )
                .unwrap()
            );

            // A
            let bob_sec = crate::nuts::SecretKey::from_hex(
                "7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f",
            )
            .unwrap();

            // C_
            let signed = sign_message(bob_sec.into(), blinded_message.into()).unwrap();

            assert_eq!(
                signed,
                k256::PublicKey::from_sec1_bytes(
                    &hex::decode(
                        "027726f0e5757b4202a27198369a3477a17bc275b7529da518fc7cb4a1d927cc0d"
                    )
                    .unwrap()
                )
                .unwrap()
            );
        }

        #[ignore]
        #[test]
        fn test_blinded_dhke() {
            // a
            let bob_sec = SecretKey::random(&mut rand::thread_rng());

            // A
            let bob_pub = bob_sec.public_key();

            // let alice_sec = SecretKey::random(&mut rand::thread_rng());

            let x = Secret::new();

            // Y
            let y = hash_to_curve(&x.to_bytes().unwrap()).unwrap();

            // B_
            let blinded = blind_message(&y.to_sec1_bytes(), None).unwrap();

            // C_
            let signed = sign_message(bob_sec.clone(), blinded.0.into()).unwrap();

            // C
            let c = unblind_message(signed.into(), blinded.1, bob_pub.into()).unwrap();

            assert!(verify_message(bob_sec, c.into(), &x).is_ok());
        }
    }
}

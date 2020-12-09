use futures_await_test::async_test;
use futures_lite::AsyncWriteExt;
use samotop_smime::SMime;

#[async_test]
async fn sign_and_encrypt() -> Result<(), Box<dyn std::error::Error>> {
    let secret = b"secret stuff";
    let mut encrypted = Vec::new();
    let mut decrypted = Vec::new();

    fn is<T: Unpin + Sync + Send>(t: T) -> T {
        t
    }

    {
        let mut sign_encrypt = is(SMime::sign_and_encrypt(
            &mut encrypted,
            "tests/data/my.key",
            "tests/data/my.crt",
            "tests/data/her.crt",
        )?);

        async_std::io::copy(&mut secret.as_ref(), &mut sign_encrypt).await?;
        sign_encrypt.close().await?;
    }

    {
        let mut decrypt_verify = is(SMime::decrypt_and_verify(
            &mut decrypted,
            "tests/data/her.key",
        )?);

        async_std::io::copy(&mut encrypted.as_slice(), &mut decrypt_verify).await?;
        decrypt_verify.close().await?;
    }

    assert_eq!(decrypted, secret);
    Ok(())
}

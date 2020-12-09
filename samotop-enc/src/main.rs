use async_macros::{join, ready};
use async_process::ChildStdin;
use log::*;
use std::task::{Context, Poll};
use std::{future::Future, pin::Pin};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    async_std::task::block_on(main_fut())
}

async fn main_fut() -> Result<(), Box<dyn std::error::Error>> {
    let mut encrypted = Vec::new();

    fn is<T: Unpin>(t: T) -> T {
        t
    }

    let sign_encrypt = is(SignEncrypt::sign_and_encrypt(
        &mut encrypted,
        "../samotop-server/Samotop.key",
        "../samotop-server/Samotop.crt",
        "../samotop-server/Samotop.crt",
    )?);

    let mut inp = b"secret stuff".as_ref();
    CopyAndClose::new(&mut inp, sign_encrypt).await?;

    println!("encrypted: {:?}", encrypted);

    Ok(())
}

#[pin_project::pin_project]
pub struct SignEncrypt<'a> {
    #[pin]
    input: Option<Pin<Box<dyn async_std::io::Write>>>,
    #[pin]
    copy: Pin<Box<dyn Future<Output = async_std::io::Result<()>> + 'a>>,
}

impl<'a> SignEncrypt<'a> {
    pub fn sign_and_encrypt<W: async_std::io::Write + 'a>(
        target: W,
        my_key: &str,
        my_cert: &str,
        her_cert: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut sign = async_process::Command::new("openssl")
            .arg("smime")
            .arg("-stream")
            .arg("-sign")
            .arg("-inkey")
            .arg(my_key)
            .arg("-signer")
            .arg(my_cert)
            .kill_on_drop(true)
            .reap_on_drop(true)
            .stdin(async_process::Stdio::piped())
            .stdout(async_process::Stdio::piped())
            .spawn()?;

        let mut encrypt = async_process::Command::new("openssl")
            .arg("smime")
            .arg("-stream")
            .arg("-encrypt")
            .arg(her_cert)
            .kill_on_drop(true)
            .reap_on_drop(true)
            .stdin(async_process::Stdio::piped())
            .stdout(async_process::Stdio::piped())
            .spawn()?;

        let sign_in = sign.stdin.take().expect("sign input");
        let sign_out = sign.stdout.take().expect("sign output");
        let encrypt_in = encrypt.stdin.take().expect("encrypt input");
        let encrypt_out = encrypt.stdout.take().expect("encrypt output");
        let copy_signed = CopyAndClose::new(sign_out, encrypt_in);
        let copy_encrypted = CopyAndClose::new(encrypt_out, target);

        let copy = async move {
            let (res1, res2) = join!(copy_signed, copy_encrypted).await;
            res1?;
            res2?;
            debug!("sign: {:?}", sign.status().await?);
            debug!("encrypt: {:?}", encrypt.status().await?);
            Ok(())
        };
        let writer = SignEncrypt {
            input: Some(Box::pin(sign_in)),
            copy: Box::pin(copy),
        };

        Ok(writer)
    }
}

impl<'a> async_std::io::Write for SignEncrypt<'a> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        let this = self.project();
        debug!("poll_write {}", buf.len());
        if let Some(i) = this.input.as_pin_mut() {
            debug!("poll_write input");
            let written = ready!(i.poll_write(cx, buf))?;
            Poll::Ready(Ok(written))
        } else {
            Poll::Ready(Err(std::io::ErrorKind::NotConnected.into()))
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        let mut this = self.project();
        debug!("poll_flush");
        if let Some(i) = this.input.as_mut().as_pin_mut() {
            debug!("poll_flush input");
            ready!(i.poll_flush(cx))?;
        }
        ready!(this.input.as_pin_mut().expect("input").poll_flush(cx))?;
        debug!("poll copy...");
        let copied = ready!(this.copy.poll(cx))?;
        debug!("poll copy = {:?}", copied);
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        let mut this = self.project();
        if let Some(i) = this.input.as_mut().as_pin_mut() {
            debug!("poll_close input");
            ready!(i.poll_close(cx))?;
        }

        // must drop input to finish processing
        this.input.set(None);
        debug!("closed input");

        debug!("close poll copy...");
        let copied = ready!(this.copy.poll(cx))?;
        debug!("close poll copy = {:?}", copied);

        Poll::Ready(Ok(()))
    }
}

// #[pin_project::pin_project]
// struct WriteAll<'a, W> {
//     pub from: &'a [u8],
//     #[pin]
//     pub to: W,
// }

// impl<W> async_std::future::Future for WriteAll<'_, W>
// where
//     W: async_std::io::Write,
// {
//     type Output = std::io::Result<()>;

//     fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
//         let mut this = self.project();
//         while !this.from.is_empty() {
//             debug!("WAF poll_write {}", this.from.len());
//             let n = match this.to.as_mut().poll_write(cx, this.from)? {
//                 Poll::Pending => return Poll::Pending,
//                 Poll::Ready(len) => len,
//             };
//             debug!("WAF poll_write => {}", n);
//             {
//                 let (_, rest) = std::mem::replace(this.from, &[]).split_at(n);
//                 *this.from = &rest[..];
//             }
//             if n == 0 {
//                 return Poll::Ready(Err(std::io::ErrorKind::WriteZero.into()));
//             }
//         }

//         debug!("WAF poll_close...");
//         this.to.as_mut().poll_close(cx)
//     }
// }
#[pin_project::pin_project]
struct CopyAndClose<R, W> {
    #[pin]
    reader: R,
    #[pin]
    writer: W,
    amt: u64,
}

impl<R, W> CopyAndClose<async_std::io::BufReader<R>, W>
where
    R: async_std::io::Read,
    W: async_std::io::Write,
{
    pub fn new(reader: R, writer: W) -> Self {
        CopyAndClose {
            reader: async_std::io::BufReader::new(reader),
            writer,
            amt: 0,
        }
    }
}

impl<R, W> async_std::future::Future for CopyAndClose<R, W>
where
    R: async_std::io::BufRead,
    W: async_std::io::Write,
{
    type Output = async_std::io::Result<u64>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
        loop {
            debug!("copy poll_fill_buf...");
            let buffer = ready!(this.reader.as_mut().poll_fill_buf(cx))?;
            debug!("copy poll_fill_buf => {}", buffer.len());
            if buffer.is_empty() {
                debug!("copy poll_close...");
                ready!(this.writer.as_mut().poll_close(cx))?;
                return Poll::Ready(Ok(*this.amt));
            }

            debug!("copy poll_write...");
            let i = ready!(this.writer.as_mut().poll_write(cx, buffer))?;
            debug!("copy poll_write => {}", i);
            if i == 0 {
                return Poll::Ready(Err(async_std::io::ErrorKind::WriteZero.into()));
            }
            *this.amt += i as u64;
            this.reader.as_mut().consume(i);
        }
    }
}

/*
fn main_old() {
    // // The password will be used to generate a key
    // let password = b"nice password";

    // // Usually the salt has some random data and something that relates to the user
    // // like an username
    // let mut salt = [0u8;16];
    // SystemRandom::new().fill(&mut salt).unwrap();

    // // Keys are sent as &[T] and must have 32 bytes
    // let mut key = [0; 32];
    // derive(
    //     PBKDF2_HMAC_SHA256,
    //     NonZeroU32::new(100).unwrap(),
    //     &salt,
    //     &password[..],
    //     &mut key,
    // );

    let mut key = [0u8; 32];
    SystemRandom::new().fill(&mut key).unwrap();

    // Your private data
    let content = b"content to encrypt".to_vec();
    println!("Content to encrypt's size {}", content.len());

    // Ring uses the same input variable as output
    let mut encrypted = content.clone();

    // The input/output variable need some space for a suffix
    println!("Tag len {}", CHACHA20_POLY1305.tag_len());
    for _ in 0..CHACHA20_POLY1305.tag_len() {
        encrypted.push(0);
    }

    struct Nonces(usize);
    impl Nonces {
        pub fn new(rand: usize) -> Self {
            Nonces(rand)
        }
    }
    impl NonceSequence for Nonces {
        fn advance(&mut self) -> Result<Nonce, Unspecified> {
            // Random data must be used only once per encryption

            let mut nonce = [self.0 as u8; 12];
            for n in nonce.iter_mut() {
                self.0 += 1;
                *n = (self.0 % 256) as u8
            }
            Ok(Nonce::assume_unique_for_key(nonce))
        }
    }

    assert_eq!(key.len(), CHACHA20_POLY1305.key_len());

    // Opening key used to decrypt data
    let mut opening_key = OpeningKey::new(
        UnboundKey::new(&CHACHA20_POLY1305, key.as_ref()).unwrap(),
        Nonces::new(5),
    );

    // Sealing key used to encrypt data
    let mut sealing_key = SealingKey::new(
        UnboundKey::new(&CHACHA20_POLY1305, key.as_ref()).unwrap(),
        Nonces::new(5),
    );

    // Encrypt data into in_out variable
    sealing_key
        .seal_in_place_append_tag(Aad::empty(), &mut encrypted)
        .unwrap();

    println!("encrypted: {}", encrypted.len());
    for b in encrypted.iter() {
        print!("{:x}", b);
    }
    println!();

    let mut decrypted = encrypted.clone();

    let decrypted_data = opening_key
        .open_in_place(Aad::empty(), &mut decrypted)
        .unwrap();
    let decrypted_data = &decrypted_data[0..decrypted_data.len() - CHACHA20_POLY1305.tag_len()];
    println!("{:?}", String::from_utf8_lossy(decrypted_data));
    assert_eq!(content, decrypted_data);

    // now this will encrypt the symetric key with the recipients public key:

    let mut rng = OsRng;
    let bits = 2048;
    let priv_key = RSAPrivateKey::new(&mut rng, bits).expect("failed to generate a key");
    let pub_key = RSAPublicKey::from(&priv_key);

    // Encrypt
    let enc_key = pub_key
        .encrypt(&mut rng, PaddingScheme::new_pkcs1v15_encrypt(), &key[..])
        .expect("failed to encrypt");
    assert_ne!(&key[..], &enc_key[..]);

    // Decrypt
    let dec_key = priv_key
        .decrypt(PaddingScheme::new_pkcs1v15_encrypt(), &enc_key)
        .expect("failed to decrypt");
    assert_eq!(&key[..], &dec_key[..]);
}

#[test]
fn enc() -> Result<(), Box<dyn std::error::Error>> {
    let pkey = PKey::private_key_from_pem(KEY)?;
    let cert = X509::from_pem(CERT)?;
    let mut certs = Stack::new()?;
    certs.push(cert.clone())?;

    let flags = Pkcs7Flags::STREAM;
    let message = b"secret stuff";

    let pkcs7 = Pkcs7::encrypt(&certs.as_ref(), message, Cipher::aes_256_cbc(), flags)?;

    let encrypted = pkcs7.to_smime(message, flags).expect("should succeed");

    std::fs::File::create("test/enc")?.write_all(encrypted.as_slice())?;

    let (pkcs7_decoded, _) = Pkcs7::from_smime(encrypted.as_slice()).expect("should succeed");

    let decoded = pkcs7_decoded
        .decrypt(&pkey, &cert, Pkcs7Flags::empty())
        .expect("should succeed");

    assert_eq!(decoded.as_slice(), message);
    Ok(())
}

const CERT: &'static [u8] = b"-----BEGIN CERTIFICATE-----
MIIFcTCCA1mgAwIBAgIUSU30P13xgqipeG1/MUfgbxSvlqQwDQYJKoZIhvcNAQEL
BQAwSDELMAkGA1UEBhMCY3oxEDAOBgNVBAgMB2N6ZWNoaWExEzARBgNVBAoMCmJy
aWdodG9wZW4xEjAQBgNVBAMMCWxvY2FsaG9zdDAeFw0yMDEyMDEwMDA0NDNaFw0y
MTEyMDEwMDA0NDNaMEgxCzAJBgNVBAYTAmN6MRAwDgYDVQQIDAdjemVjaGlhMRMw
EQYDVQQKDApicmlnaHRvcGVuMRIwEAYDVQQDDAlsb2NhbGhvc3QwggIiMA0GCSqG
SIb3DQEBAQUAA4ICDwAwggIKAoICAQCvkR9CCZ+yZWWJISDujan8Mr04Z06PnMPh
EGcSrOaRDUw0Gj6c6peQJTgy0Po2+82YTsmzz6QLr7N/PdCwsjjjGwrDM6Os7MZk
tMQDIQnLs8sxBVZpii+hD9vCUAv44xibbVg+l+iRGpAKNa062SHTFrloLE7EsVU/
fKuFBhWW6TxuU3Fxn10xZmsEjAKc5v8L6oMAY3cHLr0O8VSq2qtX5ajscUa3FB5q
2+86tkHC2npzpA+B0+BlAK4zHuUGMmhRk8ky1gjUoOH4aMa4vzHVHSO6bPs8pr1M
A775zjuo4pQBmTiG9WevyDCGymuWdHhd5oUAZ3VKF7Mbf/D49DZgIzLAC+vYGRP5
h4k/kblFpfMbx1MZHkkacyMqibsgFc47fNENFU8bW3xsPA6LgaYTjDcMaqepbepl
7G4J8HwgC0hwp2CCD2dvBMfhGzu2iOZjSydBPyB7vaL0Ei7W/ow+4+c57XZ+68XG
66QxJDf/thwkRSF0V7JVuFRDTnJ6xxP9RO7Yxroejcw1dwXv6KLvb9K6p4HNhfB+
rX3PH1pPqKKWiqAxuphU1MVweU4ni7qweMxZoAbiIodJC5ObJ+FMVhJO3hERgz1z
TohbGtTOtWw7ylJELuUQbdZJjyQn0d/dh/FAZLgTaRDTEbJcyoJZCxFZpaKsL4j4
KtbjeZWLDQIDAQABo1MwUTAdBgNVHQ4EFgQULb02drG6IKbIp+1ME0gsTJCoJoUw
HwYDVR0jBBgwFoAULb02drG6IKbIp+1ME0gsTJCoJoUwDwYDVR0TAQH/BAUwAwEB
/zANBgkqhkiG9w0BAQsFAAOCAgEAQ00Ul2lgV9m4mrP5WIcOHgIwLVjGsxE1yYgD
92YAwDS8JcM9kqakuGU0aSYG5HBn2wTtppNtF+H6fe3IACUwKN3cct9kwW/6JdTe
5W/dGWfupgUJ89KLGXneBX0RrfEkU9+7/57efu2N4Aka+ToGn+S5H95erqzeFwVT
P8H/SfWL4Vq8hMFOg8WfJCgXblyjxYQWKePZPgyxpvMkUsmIhME+yiWdpf+gfHcJ
8eeBArPii0Hf+3AKJCuhxwRiqSwlhCvcvsVCKMWOIHimJs7nIIONxYnqSQMwlwgu
k+GXAGwvQ1WZebWrntgirVRY/SAA5cww4Etdf9uFjU6DnM/PLIEYZ8YX3Wl158Y+
qNRMYQ6MgLqNjd8YsRstDVImN5KQatsRFjs0fArlEHuf0SSoGQqi/KKFjvMQU71z
7jC2OZ2/dJAB9MgwiQBJk8p/osytDcVhA5ev+3EwhcYYnjNMloT8CHYAsrl3QnDM
qWa0b5JYUQSuR5RTagGHRLuwzvdshTaE5s20hntyU8j+/vjuO0Kt4RBhA/7GYBYT
HBO8QPy8HRJUTkwj4ezh+UhA0iY7lxZrL7YJus8DqqtbECw+9O1oEy8iDWftkrRX
k9wCTgHKabKAlwPP/5orxW9bL1uAeDZx6ImlwyllL99lMSM24YvV16ngrIcAyKWl
Ob1voW0=
-----END CERTIFICATE-----
";

const KEY: &'static [u8] = b"-----BEGIN PRIVATE KEY-----
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
cQvCfN8Kp6m7nt3snN90YscqTLwPbI4=
-----END PRIVATE KEY-----
";
*/

use samotop_delivery::prelude::{EmailAddress, Envelope, SmtpClient};
use samotop_delivery::smtp::authentication::Credentials;

fn main() {
    async_std::task::block_on(async move {
        let envelope = Envelope::new(
            Some(EmailAddress::new("from@gmail.com".to_string()).unwrap()),
            vec![EmailAddress::new("to@example.com".to_string()).unwrap()],
            "id".to_string(),
        )
        .unwrap();
        let message = "Hello example".as_bytes();

        let creds = Credentials::new(
            "example_username".to_string(),
            "example_password".to_string(),
        );

        // Open a remote connection to gmail
        let mailer = SmtpClient::new("smtp.gmail.com")
            .expect("should succeed")
            .credentials(creds);

        // Send the email
        let result = mailer.connect_and_send(envelope, message).await;

        if result.is_ok() {
            println!("Email sent");
        } else {
            println!("Could not send email: {:?}", result);
        }

        assert!(result.is_ok());
    });
}

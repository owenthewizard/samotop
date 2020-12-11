use crate::{
    common::*,
    mail::{StartMailFailure, StartMailResult, Transaction},
    smtp::{SmtpPath, SmtpSessionCommand, SmtpState},
};

/// Starts new mail transaction
#[derive(Eq, PartialEq, Debug, Clone)]
pub enum SmtpMail {
    Mail(SmtpPath, Vec<String>),
    Send(SmtpPath, Vec<String>),
    Saml(SmtpPath, Vec<String>),
    Soml(SmtpPath, Vec<String>),
}

impl SmtpMail {
    pub fn verb(&self) -> &str {
        match self {
            SmtpMail::Mail(_, _) => "MAIL",
            SmtpMail::Send(_, _) => "SEND",
            SmtpMail::Saml(_, _) => "SAML",
            SmtpMail::Soml(_, _) => "SOML",
        }
    }
        pub fn sender(&self) -> &SmtpPath {
            match self {
                SmtpMail::Mail(p, _) => &p,
                SmtpMail::Send(p, _) => &p,
                SmtpMail::Saml(p, _) => &p,
                SmtpMail::Soml(p, _) => &p,
            }
        }
}

// impl SmtpSessionCommand for SmtpMail {
//     fn verb(&self) -> &str {
//         match self {
//             SmtpMail::Mail(_, _) => "MAIL",
//             SmtpMail::Send(_, _) => "SEND",
//             SmtpMail::Saml(_, _) => "SAML",
//             SmtpMail::Soml(_, _) => "SOML",
//         }
//     }

//     fn apply(&self, mut state: SmtpState) -> S2Fut<SmtpState> {
//         if state.session.peer_name.is_none() {
//             state.say_command_sequence_fail();
//             return Box::pin(ready(state));
//         }
//         state.reset();

//         let transaction = Transaction {
//             mail: Some(self.clone()),
//             ..Transaction::default()
//         };

//         let fut = async move {
//             use StartMailResult as R;
//             match state.service.start_mail(&state.session, transaction).await {
//                 R::Failed(StartMailFailure::TerminateSession, description) => {
//                     state.say_shutdown_err(description);
//                 }
//                 R::Failed(failure, description) => {
//                     state.say_mail_failed(failure, description);
//                 }
//                 R::Accepted(mut transaction) => {
//                     if transaction.id.is_empty() {
//                         fn nunnumber(input: char) -> bool {
//                             !input.is_ascii_digit()
//                         }
//                         // for the lack of better unique string without extra dependencies
//                         let id = format!("{:?}", std::time::Instant::now()).replace(nunnumber, "");
//                         warn!(
//                             "Mail transaction ID is empty. Will use time based ID {}",
//                             id
//                         );
//                         transaction.id = id;
//                     }
//                     state.say_ok_info(format!("Ok! Transaction {} started.", transaction.id));
//                     state.transaction = transaction;
//                 }
//             };
//             state
//         };

//         Box::pin(fut)
//     }
// }


// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::{
//         mail::{Builder, Recipient},
//         smtp::{CodecControl, SmtpMail, SmtpPath},
//     };
//     use futures_await_test::async_test;

//     #[async_test]
//     async fn transaction_gets_reset() {
//         let mut set = SmtpState::new(Builder::default());
//         set.session.peer_name = Some("xx.io".to_string());
//         set.transaction.id = "someid".to_owned();
//         set.transaction.mail = Some(SmtpMail::Mail(SmtpPath::Null, vec![]));
//         set.transaction.rcpts.push(Recipient::null());
//         set.transaction.extra_headers.insert_str(0, "feeeha");
//         let sut = SmtpMail::Mail(SmtpPath::Postmaster, vec![]);
//         let mut res = sut.apply(set).await;
//         match res.writes.pop_front() {
//             Some(CodecControl::Response(bytes)) if bytes.starts_with(b"250 ") => {}
//             otherwise => panic!("Expected OK, got {:?}", otherwise),
//         }
//         assert_ne!(res.transaction.id, "someid");
//         assert!(res.transaction.rcpts.is_empty());
//         assert!(res.transaction.extra_headers.is_empty());
//     }

//     #[async_test]
//     async fn mail_is_set() {
//         let mut set = SmtpState::new(Builder::default());
//         set.session.peer_name = Some("xx.io".to_string());
//         let sut = SmtpMail::Mail(SmtpPath::Postmaster, vec![]);
//         let mut res = sut.apply(set).await;
//         match res.writes.pop_front() {
//             Some(CodecControl::Response(bytes)) if bytes.starts_with(b"250 ") => {}
//             otherwise => panic!("Expected OK, got {:?}", otherwise),
//         }
//         assert_eq!(
//             res.transaction.mail,
//             Some(SmtpMail::Mail(SmtpPath::Postmaster, vec![]))
//         );
//     }

//     #[async_test]
//     async fn command_sequence_is_enforced() {
//         // MAIL command requires HELO/EHLO
//         let set = SmtpState::new(Builder::default());
//         let sut = SmtpMail::Mail(SmtpPath::Postmaster, vec![]);
//         let mut res = sut.apply(set).await;
//         match res.writes.pop_front() {
//             Some(CodecControl::Response(bytes)) if bytes.starts_with(b"503 ") => {}
//             otherwise => panic!("Expected command sequence failure, got {:?}", otherwise),
//         }
//         assert_eq!(res.transaction.mail, None);
//     }
// }

use super::*;
use crate::model::mail::*;

pub struct CompositeMailService<ES, GS, QS>((ES, GS, QS));

pub trait IntoComponents: MailService {
    type Esmtp: EsmtpService;
    type Guard: MailGuard;
    type Queue: MailQueue;
    fn into_components(self) -> (Self::Esmtp, Self::Guard, Self::Queue);
}

impl<ES, GS, QS> From<(ES, GS, QS)> for CompositeMailService<ES, GS, QS> {
    fn from(tuple: (ES, GS, QS)) -> Self {
        Self(tuple)
    }
}

impl<ES, GS, QS> IntoComponents for CompositeMailService<ES, GS, QS>
where
    ES: EsmtpService,
    GS: MailGuard,
    QS: MailQueue,
{
    type Esmtp = ES;
    type Guard = GS;
    type Queue = QS;
    fn into_components(self) -> (Self::Esmtp, Self::Guard, Self::Queue) {
        self.0
    }
}

impl<T> IntoComponents for T
where
    T: MailService + Clone,
{
    type Esmtp = Self;
    type Guard = Self;
    type Queue = Self;
    fn into_components(self) -> (Self::Esmtp, Self::Guard, Self::Queue) {
        (self.clone(), self.clone(), self)
    }
}

pub trait CompositeServiceExt: IntoComponents + Sized {
    fn replace_esmtp<T, F>(
        self,
        replacement: F,
    ) -> CompositeMailService<T, Self::Guard, Self::Queue>
    where
        T: EsmtpService,
        F: FnOnce(Self::Esmtp) -> T,
    {
        let (extend, guard, queue) = self.into_components();
        let extend = replacement(extend);
        (extend, guard, queue).into()
    }
    fn with_esmtp<T: EsmtpService>(
        self,
        esmtp: T,
    ) -> CompositeMailService<T, Self::Guard, Self::Queue> {
        self.replace_esmtp(|_| esmtp)
    }
    fn replace_guard<T, F>(
        self,
        replacement: F,
    ) -> CompositeMailService<Self::Esmtp, T, Self::Queue>
    where
        T: MailGuard,
        F: FnOnce(Self::Guard) -> T,
    {
        let (extend, guard, queue) = self.into_components();
        let guard = replacement(guard);
        (extend, guard, queue).into()
    }
    fn with_guard<T: MailGuard>(
        self,
        guard: T,
    ) -> CompositeMailService<Self::Esmtp, T, Self::Queue> {
        self.replace_guard(|_| guard)
    }
    fn replace_queue<T, F>(
        self,
        replacement: F,
    ) -> CompositeMailService<Self::Esmtp, Self::Guard, T>
    where
        T: MailQueue,
        F: FnOnce(Self::Queue) -> T,
    {
        let (extend, guard, queue) = self.into_components();
        let queue = replacement(queue);
        (extend, guard, queue).into()
    }
    fn with_queue<T: MailQueue>(
        self,
        queue: T,
    ) -> CompositeMailService<Self::Esmtp, Self::Guard, T> {
        self.replace_queue(|_| queue)
    }
}
impl<T: IntoComponents> CompositeServiceExt for T {}

impl<ES, GS, QS> EsmtpService for CompositeMailService<ES, GS, QS>
where
    ES: EsmtpService,
    GS: MailGuard,
    QS: MailQueue,
{
    fn prepare_session(&self, session: &mut SessionInfo) {
        (self.0).0.prepare_session(session)
    }
}

impl<ES, GS, QS> MailGuard for CompositeMailService<ES, GS, QS>
where
    ES: EsmtpService,
    GS: MailGuard,
    QS: MailQueue,
{
    type RecipientFuture = GS::RecipientFuture;
    type SenderFuture = GS::SenderFuture;
    fn add_recipient(&self, request: AddRecipientRequest) -> Self::RecipientFuture {
        (self.0).1.add_recipient(request)
    }
    fn start_mail(&self, request: StartMailRequest) -> Self::SenderFuture {
        (self.0).1.start_mail(request)
    }
}

impl<ES, GS, QS> MailQueue for CompositeMailService<ES, GS, QS>
where
    ES: EsmtpService,
    GS: MailGuard,
    QS: MailQueue,
{
    type Mail = QS::Mail;
    type MailFuture = QS::MailFuture;
    fn mail(&self, envelope: Envelope) -> Self::MailFuture {
        (self.0).2.mail(envelope)
    }
}

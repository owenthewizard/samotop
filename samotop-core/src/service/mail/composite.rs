use super::*;
use crate::model::mail::*;

pub struct CompositeMailService<NS, ES, GS, QS>((NS, ES, GS, QS));

pub trait IntoComponents: MailService {
    type Named: NamedService;
    type Esmtp: EsmtpService;
    type Guard: MailGuard;
    type Queue: MailQueue;
    fn into_components(self) -> (Self::Named, Self::Esmtp, Self::Guard, Self::Queue);
}

impl<NS, ES, GS, QS> From<(NS, ES, GS, QS)> for CompositeMailService<NS, ES, GS, QS> {
    fn from(tuple: (NS, ES, GS, QS)) -> Self {
        Self(tuple)
    }
}

impl<NS, ES, GS, QS> IntoComponents for CompositeMailService<NS, ES, GS, QS>
where
    NS: NamedService,
    ES: EsmtpService,
    GS: MailGuard,
    QS: MailQueue,
{
    type Named = NS;
    type Esmtp = ES;
    type Guard = GS;
    type Queue = QS;
    fn into_components(self) -> (Self::Named, Self::Esmtp, Self::Guard, Self::Queue) {
        self.0
    }
}

impl<T> IntoComponents for T
where
    T: MailService + Clone,
{
    type Named = Self;
    type Esmtp = Self;
    type Guard = Self;
    type Queue = Self;
    fn into_components(self) -> (Self::Named, Self::Esmtp, Self::Guard, Self::Queue) {
        (self.clone(), self.clone(), self.clone(), self)
    }
}

pub trait CompositeServiceExt: IntoComponents + Sized {
    fn replace_name<T, F>(
        self,
        replacement: F,
    ) -> CompositeMailService<T, Self::Esmtp, Self::Guard, Self::Queue>
    where
        T: NamedService,
        F: FnOnce(Self::Named) -> T,
    {
        let (named, extend, guard, queue) = self.into_components();
        let named = replacement(named);
        (named, extend, guard, queue).into()
    }
    fn with_name<T: NamedService>(
        self,
        named: T,
    ) -> CompositeMailService<T, Self::Esmtp, Self::Guard, Self::Queue> {
        self.replace_name(|_| named)
    }
    fn replace_esmtp<T, F>(
        self,
        replacement: F,
    ) -> CompositeMailService<Self::Named, T, Self::Guard, Self::Queue>
    where
        T: EsmtpService,
        F: FnOnce(Self::Esmtp) -> T,
    {
        let (named, extend, guard, queue) = self.into_components();
        let extend = replacement(extend);
        (named, extend, guard, queue).into()
    }
    fn with_esmtp<T: EsmtpService>(
        self,
        esmtp: T,
    ) -> CompositeMailService<Self::Named, T, Self::Guard, Self::Queue> {
        self.replace_esmtp(|_| esmtp)
    }
    fn replace_guard<T, F>(
        self,
        replacement: F,
    ) -> CompositeMailService<Self::Named, Self::Esmtp, T, Self::Queue>
    where
        T: MailGuard,
        F: FnOnce(Self::Guard) -> T,
    {
        let (named, extend, guard, queue) = self.into_components();
        let guard = replacement(guard);
        (named, extend, guard, queue).into()
    }
    fn with_guard<T: MailGuard>(
        self,
        guard: T,
    ) -> CompositeMailService<Self::Named, Self::Esmtp, T, Self::Queue> {
        self.replace_guard(|_| guard)
    }
    fn replace_queue<T, F>(
        self,
        replacement: F,
    ) -> CompositeMailService<Self::Named, Self::Esmtp, Self::Guard, T>
    where
        T: MailQueue,
        F: FnOnce(Self::Queue) -> T,
    {
        let (named, extend, guard, queue) = self.into_components();
        let queue = replacement(queue);
        (named, extend, guard, queue).into()
    }
    fn with_queue<T: MailQueue>(
        self,
        queue: T,
    ) -> CompositeMailService<Self::Named, Self::Esmtp, Self::Guard, T> {
        self.replace_queue(|_| queue)
    }
}
impl<T: IntoComponents> CompositeServiceExt for T {}

impl<NS, ES, GS, QS> NamedService for CompositeMailService<NS, ES, GS, QS>
where
    NS: NamedService,
    ES: EsmtpService,
    GS: MailGuard,
    QS: MailQueue,
{
    fn name(&self) -> &str {
        (self.0).0.name()
    }
}

impl<NS, ES, GS, QS> EsmtpService for CompositeMailService<NS, ES, GS, QS>
where
    NS: NamedService,
    ES: EsmtpService,
    GS: MailGuard,
    QS: MailQueue,
{
    fn extend(&self, connection: &mut SessionInfo) {
        (self.0).1.extend(connection)
    }
}

impl<NS, ES, GS, QS> MailGuard for CompositeMailService<NS, ES, GS, QS>
where
    NS: NamedService,
    ES: EsmtpService,
    GS: MailGuard,
    QS: MailQueue,
{
    type RecipientFuture = GS::RecipientFuture;
    type SenderFuture = GS::SenderFuture;
    fn add_recipient(&self, request: AddRecipientRequest) -> Self::RecipientFuture {
        (self.0).2.add_recipient(request)
    }
    fn start_mail(&self, request: StartMailRequest) -> Self::SenderFuture {
        (self.0).2.start_mail(request)
    }
}

impl<NS, ES, GS, QS> MailQueue for CompositeMailService<NS, ES, GS, QS>
where
    NS: NamedService,
    ES: EsmtpService,
    GS: MailGuard,
    QS: MailQueue,
{
    type Mail = QS::Mail;
    type MailFuture = QS::MailFuture;
    fn mail(&self, envelope: Envelope) -> Self::MailFuture {
        (self.0).3.mail(envelope)
    }
}

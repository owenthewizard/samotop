use super::*;
use crate::model::mail::*;

pub struct CompositeMailService<ES, GS, DS>((ES, GS, DS));

pub trait IntoComponents: MailService {
    type Esmtp: EsmtpService;
    type Guard: MailGuard;
    type Dispatch: MailDispatch;
    fn into_components(self) -> (Self::Esmtp, Self::Guard, Self::Dispatch);
}

impl<ES, GS, DS> From<(ES, GS, DS)> for CompositeMailService<ES, GS, DS> {
    fn from(tuple: (ES, GS, DS)) -> Self {
        Self(tuple)
    }
}

impl<ES, GS, DS> IntoComponents for CompositeMailService<ES, GS, DS>
where
    ES: EsmtpService,
    GS: MailGuard,
    DS: MailDispatch,
{
    type Esmtp = ES;
    type Guard = GS;
    type Dispatch = DS;
    fn into_components(self) -> (Self::Esmtp, Self::Guard, Self::Dispatch) {
        self.0
    }
}

impl<T> IntoComponents for T
where
    T: MailService + Clone,
{
    type Esmtp = Self;
    type Guard = Self;
    type Dispatch = Self;
    fn into_components(self) -> (Self::Esmtp, Self::Guard, Self::Dispatch) {
        (self.clone(), self.clone(), self)
    }
}

pub trait CompositeServiceExt: IntoComponents + Sized {
    fn replace_esmtp<T, F>(
        self,
        replacement: F,
    ) -> CompositeMailService<T, Self::Guard, Self::Dispatch>
    where
        T: EsmtpService,
        F: FnOnce(Self::Esmtp) -> T,
    {
        let (extend, guard, dispatch) = self.into_components();
        let extend = replacement(extend);
        (extend, guard, dispatch).into()
    }
    fn with_esmtp<T: EsmtpService>(
        self,
        esmtp: T,
    ) -> CompositeMailService<T, Self::Guard, Self::Dispatch> {
        self.replace_esmtp(|_| esmtp)
    }
    fn replace_guard<T, F>(
        self,
        replacement: F,
    ) -> CompositeMailService<Self::Esmtp, T, Self::Dispatch>
    where
        T: MailGuard,
        F: FnOnce(Self::Guard) -> T,
    {
        let (extend, guard, dispatch) = self.into_components();
        let guard = replacement(guard);
        (extend, guard, dispatch).into()
    }
    fn with_guard<T: MailGuard>(
        self,
        guard: T,
    ) -> CompositeMailService<Self::Esmtp, T, Self::Dispatch> {
        self.replace_guard(|_| guard)
    }
    fn replace_dispatch<T, F>(
        self,
        replacement: F,
    ) -> CompositeMailService<Self::Esmtp, Self::Guard, T>
    where
        T: MailDispatch,
        F: FnOnce(Self::Dispatch) -> T,
    {
        let (extend, guard, dispatch) = self.into_components();
        let dispatch = replacement(dispatch);
        (extend, guard, dispatch).into()
    }
    fn with_dispatch<T: MailDispatch>(
        self,
        dispatch: T,
    ) -> CompositeMailService<Self::Esmtp, Self::Guard, T> {
        self.replace_dispatch(|_| dispatch)
    }
}
impl<T: IntoComponents> CompositeServiceExt for T {}

impl<ES, GS, DS> EsmtpService for CompositeMailService<ES, GS, DS>
where
    ES: EsmtpService,
    GS: MailGuard,
    DS: MailDispatch,
{
    fn prepare_session(&self, session: &mut SessionInfo) {
        (self.0).0.prepare_session(session)
    }
}

impl<ES, GS, DS> MailGuard for CompositeMailService<ES, GS, DS>
where
    ES: EsmtpService,
    GS: MailGuard,
    DS: MailDispatch,
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

impl<ES, GS, DS> MailDispatch for CompositeMailService<ES, GS, DS>
where
    ES: EsmtpService,
    GS: MailGuard,
    DS: MailDispatch,
{
    type Mail = DS::Mail;
    type MailFuture = DS::MailFuture;
    fn send_mail(&self, transaction: Transaction) -> Self::MailFuture {
        (self.0).2.send_mail(transaction)
    }
}

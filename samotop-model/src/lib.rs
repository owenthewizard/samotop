pub mod io;
pub mod mail;
pub mod smtp;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, PartialEq, Eq, Clone)]
struct DefaultMailService;
impl NamedService for DefaultMailService {}
impl EsmtpService for DefaultMailService {}
impl MailGuard for DefaultMailService {}
impl MailQueue for DefaultMailService {}

trait NamedService {}
trait EsmtpService {}
trait MailGuard {}
trait MailQueue {}
trait MailService: NamedService + EsmtpService + MailGuard + MailQueue + Sized {
    fn using<MS: MailSetup<Self>>(self, setup: MS) -> MS::Output {
        setup.setup(self)
    }
}
impl<T> MailService for T where T: NamedService + EsmtpService + MailGuard + MailQueue + Sized {}

trait MailSetup<X> {
    type Output: MailService;
    fn setup(&self, what: X) -> Self::Output;
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct CompositeMailService<NS, ES, GS, QS> {
    a: NS,
    b: ES,
    c: GS,
    d: QS,
}

impl<NS, ES, GS, QS> NamedService for CompositeMailService<NS, ES, GS, QS> where NS: NamedService {}
impl<NS, ES, GS, QS> EsmtpService for CompositeMailService<NS, ES, GS, QS> where ES: EsmtpService {}
impl<NS, ES, GS, QS> MailGuard for CompositeMailService<NS, ES, GS, QS> where GS: MailGuard {}
impl<NS, ES, GS, QS> MailQueue for CompositeMailService<NS, ES, GS, QS> where QS: MailQueue {}

struct Setup;
use std::ops::Deref;
use std::sync::Arc;

impl<A> MailSetup<A> for Setup
where
    A: MailService,
{
    type Output = CompositeMailService<Arc<A>, Arc<A>, Arc<A>, DefaultMailService>;
    fn setup(&self, what: A) -> Self::Output {
        let arc = Arc::new(what);
        CompositeMailService {
            a: arc.clone(),
            b: arc.clone(),
            c: arc,
            d: DefaultMailService,
        }
    }
}

impl<T> NamedService for T
where
    T: Deref,
    T::Target: NamedService,
{
}

impl<T> EsmtpService for T
where
    T: Deref,
    T::Target: EsmtpService,
{
}

impl<T> MailGuard for T
where
    T: Deref,
    T::Target: MailGuard,
{
}

impl<T> MailQueue for T
where
    T: Deref,
    T::Target: MailQueue,
{
}

#[test]
fn test_setup() {
    let setup = Setup;
    let svc = DefaultMailService;
    let composite = setup.setup(svc);
    hungry(composite);
}
#[test]
fn test_using() {
    let setup = Setup;
    let svc = DefaultMailService;
    let composite = svc.using(setup);
    hungry(composite);
}

fn hungry(_svc: impl MailService + Send + Sync + Clone + std::fmt::Debug + 'static) {}

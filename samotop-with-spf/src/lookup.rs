use async_std::future::timeout;
use async_std_resolver::{resolver_from_system_conf, AsyncStdResolver, ResolveError};
use std::pin::Pin;
use std::{
    future::Future,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    time::Duration,
};
use trust_dns_resolver::error::ResolveErrorKind;
use viaspf::{Lookup, LookupError, LookupResult, Name};

pub struct TrustDnsResolver {
    inner: AsyncStdResolver,
    timeout: Duration,
}

impl TrustDnsResolver {
    pub fn new(inner: AsyncStdResolver) -> Self {
        Self {
            inner,
            timeout: Duration::from_secs(5),
        }
    }
}

pub async fn new_resolver() -> Result<TrustDnsResolver, ResolveError> {
    Ok(TrustDnsResolver::new(resolver_from_system_conf().await?))
}

impl Lookup for TrustDnsResolver {
    fn lookup_a<'s, 'n, 'f>(
        &'s self,
        name: &'n Name,
    ) -> Pin<Box<dyn Future<Output = LookupResult<Vec<Ipv4Addr>>> + Send + 'f>>
    where
        's: 'f,
        'n: 'f,
    {
        Box::pin(async move {
            Ok(
                query_async(self.timeout, self.inner.ipv4_lookup(name.as_str()))
                    .await?
                    .into_iter()
                    .collect(),
            )
        })
    }

    fn lookup_aaaa<'s, 'n, 'f>(
        &'s self,
        name: &'n Name,
    ) -> Pin<Box<dyn Future<Output = LookupResult<Vec<Ipv6Addr>>> + Send + 'f>>
    where
        's: 'f,
        'n: 'f,
    {
        Box::pin(async move {
            Ok(
                query_async(self.timeout, self.inner.ipv6_lookup(name.as_str()))
                    .await?
                    .into_iter()
                    .collect(),
            )
        })
    }
    fn lookup_mx<'s, 'n, 'f>(
        &'s self,
        name: &'n Name,
    ) -> Pin<Box<dyn Future<Output = LookupResult<Vec<Name>>> + Send + 'f>>
    where
        's: 'f,
        'n: 'f,
    {
        Box::pin(async move {
            let mut mxs: Vec<_> = query_async(self.timeout, self.inner.mx_lookup(name.as_str()))
                .await?
                .into_iter()
                .collect();
            mxs.sort_by_key(|mx| mx.preference());
            mxs.into_iter()
                .map(|mx| {
                    Name::new(&mx.exchange().to_ascii())
                        .map_err(|e| LookupError::Dns(Some(e.into())))
                })
                .collect::<Result<_, _>>()
        })
    }
    fn lookup_txt<'s, 'n, 'f>(
        &'s self,
        name: &'n Name,
    ) -> Pin<Box<dyn Future<Output = LookupResult<Vec<String>>> + Send + 'f>>
    where
        's: 'f,
        'n: 'f,
    {
        Box::pin(async move {
            Ok(
                query_async(self.timeout, self.inner.txt_lookup(name.as_str()))
                    .await?
                    .into_iter()
                    .map(|data| data.to_string())
                    .collect(),
            )
        })
    }

    fn lookup_ptr<'s, 'f>(
        &'s self,
        ip: IpAddr,
    ) -> Pin<Box<dyn Future<Output = LookupResult<Vec<Name>>> + Send + 'f>>
    where
        's: 'f,
    {
        Box::pin(async move {
            query_async(self.timeout, self.inner.reverse_lookup(ip))
                .await?
                .into_iter()
                .map(|name| {
                    Name::new(&name.to_ascii()).map_err(|e| LookupError::Dns(Some(e.into())))
                })
                .collect::<Result<_, _>>()
        })
    }
}

fn query_async<'a, 'f, T>(
    time_out: Duration,
    fut: impl Future<Output = Result<T, ResolveError>> + Send + 'a,
) -> Pin<Box<dyn Future<Output = LookupResult<T>> + Send + 'f>>
where
    'a: 'f,
{
    Box::pin(async move {
        match timeout(time_out, fut).await {
            Ok(r) => r.map_err(to_lookup_error),
            Err(_timeout) => Err(LookupError::Timeout),
        }
    })
}

fn to_lookup_error(error: ResolveError) -> LookupError {
    use ResolveErrorKind::*;
    match error.kind() {
        NoRecordsFound { .. } => LookupError::NoRecords,
        Io(_) => LookupError::Dns(Some(error.into())),
        Timeout => LookupError::Timeout,
        _ => LookupError::Dns(Some(error.into())),
    }
}

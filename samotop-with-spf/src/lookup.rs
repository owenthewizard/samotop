use async_std::{future::timeout, task::block_on};
use async_std_resolver::{config, resolver, AsyncStdResolver, ResolveError};
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
    Ok(TrustDnsResolver::new(
        resolver(
            config::ResolverConfig::default(),
            config::ResolverOpts::default(),
        )
        .await?,
    ))
}

impl Lookup for TrustDnsResolver {
    fn lookup_a(&self, name: &Name) -> LookupResult<Vec<Ipv4Addr>> {
        query_sync(self.timeout, self.inner.ipv4_lookup(name.as_str()))
            .map(|lookup| lookup.into_iter().collect())
    }

    fn lookup_aaaa(&self, name: &Name) -> LookupResult<Vec<Ipv6Addr>> {
        query_sync(self.timeout, self.inner.ipv6_lookup(name.as_str()))
            .map(|lookup| lookup.into_iter().collect())
    }

    fn lookup_mx(&self, name: &Name) -> LookupResult<Vec<Name>> {
        let mut mxs: Vec<_> = query_sync(self.timeout, self.inner.mx_lookup(name.as_str()))
            .map(|lookup| lookup.into_iter().collect())?;
        mxs.sort_by_key(|mx| mx.preference());
        mxs.into_iter()
            .map(|mx| {
                Name::new(&mx.exchange().to_ascii()).map_err(|e| LookupError::Dns(Some(e.into())))
            })
            .collect::<Result<_, _>>()
    }

    fn lookup_txt(&self, name: &Name) -> LookupResult<Vec<String>> {
        let txts: Vec<_> = query_sync(self.timeout, self.inner.txt_lookup(name.as_str()))
            .map(|lookup| lookup.into_iter().collect())?;
        Ok(txts.into_iter().map(|data| data.to_string()).collect())
    }

    fn lookup_ptr(&self, ip: IpAddr) -> LookupResult<Vec<Name>> {
        let revs: Vec<_> = query_sync(self.timeout, self.inner.reverse_lookup(ip))
            .map(|lookup| lookup.into_iter().collect())?;
        revs.into_iter()
            .map(|name| Name::new(&name.to_ascii()).map_err(|e| LookupError::Dns(Some(e.into()))))
            .collect()
    }
}

fn query_sync<T>(
    time_out: Duration,
    fut: impl Future<Output = Result<T, ResolveError>>,
) -> LookupResult<T> {
    match block_on(timeout(time_out, fut)) {
        Ok(r) => r.map_err(to_lookup_error),
        Err(_) => Err(LookupError::Timeout),
    }
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

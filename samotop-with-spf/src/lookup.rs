use async_std::task::block_on;
use async_std_resolver::{config, proto::rr::rdata::MX, resolver, AsyncStdResolver, ResolveError};
use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    str,
};
use trust_dns_resolver::error::ResolveErrorKind;
use viaspf::{Lookup, LookupError, LookupResult, Name};

pub struct TrustDnsResolver(AsyncStdResolver);

impl TrustDnsResolver {
    pub fn new(resolver: AsyncStdResolver) -> Self {
        Self(resolver)
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
        let fut = self.0.ipv4_lookup(name.as_str());
        let res = block_on(fut);
        Ok(res.map_err(to_lookup_error)?.into_iter().collect())
    }

    fn lookup_aaaa(&self, name: &Name) -> LookupResult<Vec<Ipv6Addr>> {
        let fut = self.0.ipv6_lookup(name.as_str());
        let res = block_on(fut);
        Ok(res.map_err(to_lookup_error)?.into_iter().collect())
    }

    fn lookup_mx(&self, name: &Name) -> LookupResult<Vec<Name>> {
        let fut = self.0.mx_lookup(name.as_str());
        let res = block_on(fut);
        let mut mxs: Vec<MX> = res.map_err(to_lookup_error)?.into_iter().collect();
        mxs.sort_by_key(|mx| mx.preference());
        Ok(mxs
            .into_iter()
            .map(|mx| {
                Name::new(&mx.exchange().to_ascii()).map_err(|e| LookupError::Dns(Some(e.into())))
            })
            .collect::<Result<_, _>>()?)
    }

    fn lookup_txt(&self, name: &Name) -> LookupResult<Vec<String>> {
        let fut = self.0.txt_lookup(name.as_str());
        let res = block_on(fut);
        Ok(res
            .map_err(to_lookup_error)?
            .into_iter()
            .map(|txt| {
                txt.iter()
                    .map(|data| str::from_utf8(data).map_err(|e| LookupError::Dns(Some(e.into()))))
                    .collect()
            })
            .collect::<Result<_, _>>()?)
    }

    fn lookup_ptr(&self, ip: IpAddr) -> LookupResult<Vec<Name>> {
        let fut = self.0.reverse_lookup(ip);
        let res = block_on(fut);
        Ok(res
            .map_err(to_lookup_error)?
            .into_iter()
            .map(|name| Name::new(&name.to_ascii()).map_err(|e| LookupError::Dns(Some(e.into()))))
            .collect::<Result<_, _>>()?)
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

use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    str,
};
use trust_dns_resolver::{
    error::{ResolveError, ResolveErrorKind},
    Resolver,
};
use viaspf::{Lookup, LookupError, LookupResult, Name};

pub struct TrustDnsResolver(Resolver);

impl TrustDnsResolver {
    pub fn new(resolver: Resolver) -> Self {
        Self(resolver)
    }
}

impl Default for TrustDnsResolver {
    fn default() -> Self {
        TrustDnsResolver::new(Resolver::default().unwrap())
    }
}

impl Lookup for TrustDnsResolver {
    fn lookup_a(&self, name: &Name) -> LookupResult<Vec<Ipv4Addr>> {
        Ok(self
            .0
            .ipv4_lookup(name.as_str())
            .map_err(to_lookup_error)?
            .into_iter()
            .collect())
    }

    fn lookup_aaaa(&self, name: &Name) -> LookupResult<Vec<Ipv6Addr>> {
        Ok(self
            .0
            .ipv6_lookup(name.as_str())
            .map_err(to_lookup_error)?
            .into_iter()
            .collect())
    }

    fn lookup_mx(&self, name: &Name) -> LookupResult<Vec<Name>> {
        let mut mxs = self
            .0
            .mx_lookup(name.as_str())
            .map_err(to_lookup_error)?
            .into_iter()
            .collect::<Vec<_>>();
        mxs.sort_by_key(|mx| mx.preference());
        Ok(mxs
            .into_iter()
            .map(|mx| {
                Name::new(&mx.exchange().to_ascii())
                    .map_err(|e| LookupError::Protocol(Some(e.into())))
            })
            .collect::<Result<_, _>>()?)
    }

    fn lookup_txt(&self, name: &Name) -> LookupResult<Vec<String>> {
        Ok(self
            .0
            .txt_lookup(name.as_str())
            .map_err(to_lookup_error)?
            .into_iter()
            .map(|txt| {
                txt.iter()
                    .map(|data| {
                        str::from_utf8(data).map_err(|e| LookupError::Protocol(Some(e.into())))
                    })
                    .collect()
            })
            .collect::<Result<_, _>>()?)
    }

    fn lookup_ptr(&self, ip: IpAddr) -> LookupResult<Vec<Name>> {
        Ok(self
            .0
            .reverse_lookup(ip)
            .map_err(to_lookup_error)?
            .into_iter()
            .map(|name| {
                Name::new(&name.to_ascii()).map_err(|e| LookupError::Protocol(Some(e.into())))
            })
            .collect::<Result<_, _>>()?)
    }
}

fn to_lookup_error(error: ResolveError) -> LookupError {
    use ResolveErrorKind::*;
    match error.kind() {
        NoRecordsFound { .. } => LookupError::NoRecords,
        Io(_) => LookupError::Protocol(Some(error.into())),
        Timeout => LookupError::Timeout,
        _ => LookupError::Protocol(Some(error.into())),
    }
}

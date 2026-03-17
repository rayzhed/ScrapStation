/// Check if a host matches any domain in the list (including www. and subdomains)
pub fn is_matching_domain(host: &str, domains: &[String]) -> bool {
    let host_lower = host.to_lowercase();
    domains.iter().any(|domain| {
        let domain_lower = domain.to_lowercase();
        host_lower == domain_lower
            || host_lower == format!("www.{}", domain_lower)
            || host_lower.ends_with(&format!(".{}", domain_lower))
    })
}


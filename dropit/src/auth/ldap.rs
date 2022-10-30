use ldap3::{ldap_escape, Ldap, LdapConnAsync, LdapError, Scope, SearchEntry};

pub struct LdapAuthenticator {
    address: String,
    process: LdapAuthProcess,
}

impl LdapAuthenticator {
    pub fn new(address: String, process: LdapAuthProcess) -> Self {
        Self { address, process }
    }

    pub async fn is_authorized(&self, username: &str, password: &str) -> Result<bool, LdapError> {
        let (conn, mut ldap) = LdapConnAsync::new(&self.address).await?;
        ldap3::drive!(conn);

        let bind_dn = match self.process.resolve_dn(&mut ldap, username).await? {
            None => return Ok(false),
            Some(bind_dn) => bind_dn,
        };
        let res = ldap.simple_bind(&bind_dn, password).await?;

        Ok(res.success().is_ok())
    }
}

pub enum LdapAuthProcess {
    SingleBind {
        dn_pattern: String,
    },
    SearchThenBind {
        search_credentials: Option<(String, String)>,
        base_dn: String,
        filter_pattern: String,
    },
}

impl LdapAuthProcess {
    async fn resolve_dn(
        &self,
        ldap: &mut Ldap,
        username: &str,
    ) -> Result<Option<String>, LdapError> {
        match self {
            LdapAuthProcess::SingleBind { dn_pattern } => {
                Ok(Some(dn_pattern.replace("%u", &ldap_escape(username))))
            }
            LdapAuthProcess::SearchThenBind {
                search_credentials,
                base_dn,
                filter_pattern,
            } => {
                if let Some((search_username, search_password)) = search_credentials {
                    ldap.simple_bind(search_username, search_password).await?;
                }
                let (mut entries, _res) = ldap
                    .search(
                        base_dn,
                        Scope::Subtree,
                        &filter_pattern.replace("%u", &ldap_escape(username)),
                        vec![""],
                    )
                    .await?
                    .success()?;

                if entries.len() != 1 {
                    return Ok(None);
                }

                Ok(Some(SearchEntry::construct(entries.pop().unwrap()).dn))
            }
        }
    }
}

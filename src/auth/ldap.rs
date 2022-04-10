use ldap3::{ldap_escape, LdapConnAsync, LdapError, Scope, SearchEntry};

pub struct LdapAuthenticator {
    address: String,
    search_credentials: Option<(String, String)>,
    base_dn: String,
    attribute: String,
}

impl LdapAuthenticator {
    pub fn new(
        address: String,
        search_credentials: Option<(String, String)>,
        base_dn: String,
        attribute: String,
    ) -> Self {
        Self {
            address,
            search_credentials,
            base_dn,
            attribute,
        }
    }

    pub async fn is_authorized(&self, username: &str, password: &str) -> Result<bool, LdapError> {
        let (conn, mut ldap) = LdapConnAsync::new(&self.address).await?;
        ldap3::drive!(conn);

        if let Some((search_username, search_password)) = &self.search_credentials {
            ldap.simple_bind(search_username, search_password).await?;
        }
        let (mut entries, _res) = ldap
            .search(
                &self.base_dn,
                Scope::Subtree,
                &format!("({}={})", &self.attribute, ldap_escape(username)),
                vec![""],
            )
            .await?
            .success()?;

        if entries.len() != 1 {
            return Ok(false);
        }
        let entry = SearchEntry::construct(entries.pop().unwrap());
        let res = ldap.simple_bind(&entry.dn, password).await?;

        Ok(res.rc != 49)
    }
}

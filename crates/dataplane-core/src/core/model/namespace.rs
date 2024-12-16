pub struct Namespace<'a>(&'a str);

impl<'a> Namespace<'a> {
    pub fn to_iri(&self, term: &str) -> String {
        format!("{}{}", self.0, term)
    }

    pub fn ns(&self) -> &'a str {
        self.0
    }
}

pub static EDC_NAMESPACE: Namespace<'static> = Namespace("https://w3id.org/edc/v0.0.1/ns/");

pub static DSPACE_NAMESPACE: Namespace<'static> = Namespace("https://w3id.org/dspace/v0.8/");

pub static IDSA_NAMESPACE: Namespace<'static> = Namespace("https://w3id.org/idsa/v4.1/");

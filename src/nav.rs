use crate::config::Config;

use crate::{docs_finder, navigation, Result};

pub struct NavigationCommand {}

#[derive(Default)]
pub struct NavigationOptions {}

impl NavigationCommand {
    pub fn run(config: Config) -> Result<()> {
        let docs = docs_finder::find(&config);
        let nav = navigation::Navigation::new(&config);
        let tree = nav.links(&docs, false);

        println!("{}", serde_yaml::to_string(&tree).unwrap());
        Ok(())
    }
}

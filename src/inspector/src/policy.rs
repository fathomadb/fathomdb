use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Policy {
    pub(crate) no_show_inactive: bool,
    pub(crate) show_erased: bool,
    pub(crate) show_payloads: bool,
}

#[derive(Clone, Copy)]
pub(crate) struct Classification {
    pub(crate) inactive: bool,
    pub(crate) erased: bool,
    pub(crate) telemetry: bool,
    pub(crate) known: bool,
}

impl Policy {
    pub(crate) fn parse(args: &[String]) -> Result<(Self, Option<String>), &'static str> {
        let mut policy = Self::default();
        let mut path = None;
        let mut positional = false;
        for arg in args {
            if !positional {
                match arg.as_str() {
                    "--no-show-inactive" => {
                        policy.no_show_inactive = true;
                        continue;
                    }
                    "--show-erased" => {
                        policy.show_erased = true;
                        continue;
                    }
                    "--show-payloads" => {
                        policy.show_payloads = true;
                        continue;
                    }
                    "--" => {
                        positional = true;
                        continue;
                    }
                    flag if flag.starts_with('-') => return Err("invalid_arguments"),
                    _ => {}
                }
            }
            if path.replace(arg.clone()).is_some() {
                return Err("invalid_arguments");
            }
        }
        Ok((policy, path))
    }
    pub(crate) fn permits(self, class: Classification) -> bool {
        class.known
            && (!class.inactive || !self.no_show_inactive)
            && (!class.erased || self.show_erased)
            && (!class.telemetry || self.show_payloads)
    }
}

/*
Copyright ⓒ 2017 contributors.
Licensed under the MIT license (see LICENSE or <http://opensource.org
/licenses/MIT>) or the Apache License, Version 2.0 (see LICENSE of
<http://www.apache.org/licenses/LICENSE-2.0>), at your option. All
files in the project carrying such notice may not be copied, modified,
or distributed except according to those terms.
*/
macro_rules! cargo {
    ($($args:expr),* $(,)*) => {
        {
            use std::process::Command;

            let cmd_str;
            let out = {
                let mut cmd = Command::new(::std::env::var("CARGO")
                    .unwrap_or_else(|_| "cargo".into()));
                $(
                    cmd.arg($args);
                )*

                cmd_str = format!("{:?}", cmd);

                cmd.output()
                    .map(::util::Output::from)
            };

            println!("cargo cmd: {}", cmd_str);
            out
        }
    };
}

pub struct Output {
    pub status: ::std::process::ExitStatus,
    pub stdout: String,
    pub stderr: String,
}

impl Output {
    pub fn succeeded(&self, msg: &str) {
        if !self.success() {
            println!("status: {:?}", self.status);
            println!("stdout:");
            println!("-----");
            println!("{}", self.stdout);
            println!("-----");
            println!("stderr:");
            println!("-----");
            println!("{}", self.stderr);
            println!("-----");
            panic!("Command failed: {}", msg);
        }
    }

    pub fn success(&self) -> bool {
        self.status.success()
    }
}

impl From<::std::process::Output> for Output {
    fn from(v: ::std::process::Output) -> Self {
        Output {
            status: v.status,
            stdout: String::from_utf8(v.stdout).unwrap(),
            stderr: String::from_utf8(v.stderr).unwrap(),
        }
    }
}

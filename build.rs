use quote::{format_ident, quote};
use std::env;
use std::io::Write;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    //bindings
    {
        let out_dir = env::var_os("OUT_DIR").unwrap();
        let dest_path = Path::new(&out_dir).join("binding.rs");
        let mut f = std::fs::File::create(&dest_path)?;
        let variants = [
            ("Bool", "bool"),
            ("U8", "u8"),
            ("USize", "usize"),
            ("ISize", "isize"),
            ("Float", "float"),
            ("ClockData", "clock_data"),
        ];

        let mut get_type_name_variants = Vec::new();
        let mut set_type_name_variants = Vec::new();

        let mut try_bind_variants = Vec::new();
        for (var, fname) in variants.iter() {
            let g = format_ident!("as_{}_get", fname);
            let s = format_ident!("as_{}_set", fname);
            let i = format_ident!("{}", var);
            try_bind_variants.push(quote! {
                Self::Get(ParamGet::#i(p)) => {
                    if let Some(g) = binding.#g() {
                        p.bind(g);
                        Ok(())
                    } else {
                        Err(BindingError::NoGet)
                    }
                }
                Self::Set(ParamSet::#i(p)) => {
                    if let Some(s) = binding.#s() {
                        p.bind(s);
                        Ok(())
                    } else {
                        Err(BindingError::NoSet)
                    }
                }
            });
            //build up GetSet, with get from the curent, and set from each of the variants
            for (ovar, ofname) in variants.iter() {
                let s = format_ident!("as_{}_set", ofname);
                let oi = format_ident!("{}", ovar);
                try_bind_variants.push(quote! {
                    Self::GetSet(ParamGet::#i(pg), ParamSet::#oi(ps)) => {
                        if let Some(g) = binding.#g() {
                            if let Some(s) = binding.#s() {
                                pg.bind(g);
                                ps.bind(s);
                                Ok(())
                            } else {
                                Err(BindingError::NoSet)
                            }
                        } else {
                            Err(BindingError::NoGet)
                        }
                    }
                });
            }

            //build up get and set
            get_type_name_variants.push(quote! {
                ParamGet::#i(_) => #fname
            });
            set_type_name_variants.push(quote! {
                ParamSet::#i(_) => #fname
            });
        }

        f.write_all(
            quote! {
                impl ParamAccess {
                    fn as_get(&self) -> Option<&ParamGet> {
                        match self {
                            Self::Get(m) => Some(m),
                            Self::Set(_) => None,
                            Self::GetSet(m, _) => Some(m),
                        }
                    }

                    fn as_set(&self) -> Option<&ParamSet> {
                        match self {
                            Self::Get(_) => None,
                            Self::Set(m) => Some(m),
                            Self::GetSet(_, m) => Some(m),
                        }
                    }

                    ///Get the type name for the contained `Get` value, if there is one.
                    pub fn get_type_name(&self) -> Option<&str> {
                        if let Some(g) = self.as_get() {
                            Some(match g {
                                #(#get_type_name_variants,)*
                            })
                        } else {
                            None
                        }
                    }

                    ///Get the type name for the contained `Set` value, if there is one.
                    pub fn set_type_name(&self) -> Option<&str> {
                        if let Some(g) = self.as_set() {
                            Some(match g {
                                #(#set_type_name_variants,)*
                            })
                        } else {
                            None
                        }
                    }

                    /// attempt to bind.
                    pub fn try_bind(&mut self, binding: &Binding) -> Result<(), BindingError> {
                        match self {
                            #(#try_bind_variants)*
                        }
                    }

                    ///Get a `&str` representing the type of access: `"get", "set" or "getset"`
                    pub fn access_name(&self) -> &str {
                        match self {
                            Self::Get(_) => "get",
                            Self::Set(_) => "set",
                            Self::GetSet(_, _) => "getset",
                        }
                    }
                }
            }
            .to_string()
            .as_bytes(),
        )?;
    }
    println!("cargo:rerun-if-changed=build.rs");
    Ok(())
}

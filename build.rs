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

        let mut p = Vec::new();
        for (var, fname) in variants.iter() {
            let g = format_ident!("as_{}_get", fname);
            let s = format_ident!("as_{}_set", fname);
            let i = format_ident!("{}", var);
            p.push(quote! {
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
                p.push(quote! {
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
        }

        f.write_all(
            quote! {
                impl ParamAccess {
                    pub fn bind(&mut self, binding: &Binding) -> Result<(), BindingError> {
                        match self {
                            #(#p)*
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

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::env;
use std::io::Write;
use std::path::Path;

struct DataType {
    pub var_name: syn::Ident,
    pub func_name: &'static str,
    pub typ: syn::Type,
    pub type_name: String
}

struct DataAccess {
    pub enum_name: syn::Ident,
    pub trait_name: syn::Ident,
    pub access_var: syn::Ident,
    pub access_name: &'static str,
    pub type_name_prefix: &'static str,
    pub type_name_suffix: &'static str,
    pub dynamic: bool
}

impl DataType {
    pub fn new(var_name: &'static str, func_name: &'static str, typ: &'static str) -> Self {
        Self {
            var_name: format_ident!("{}", var_name),
            func_name,
            type_name: typ.split("::").last().unwrap().to_string(),
            typ: syn::parse_str(typ).unwrap(),
        }
    }
}

impl DataAccess {
    pub fn new(access_var: &'static str, trait_name: &'static str, dynamic: bool) -> Self {
        Self {
            enum_name: format_ident!("ParamData{}", access_var),
            trait_name: format_ident!("{}", trait_name),
            access_var: format_ident!("{}", access_var),
            access_name: if access_var.contains("GetSet") { "getset" } else if access_var.contains("Set") { "set" } else { "get" },
            type_name_prefix: if access_var.contains("KeyValue") { "KeyValue<" } else { "" },
            type_name_suffix: if access_var.contains("KeyValue") { ">" } else { "" },
            dynamic
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = env::var_os("OUT_DIR").unwrap();

    let mut bindings_file = std::fs::File::create(&Path::new(&out_dir).join("binding.rs"))?;
    let mut instance_factory_file = std::fs::File::create(&Path::new(&out_dir).join("instance_factory.rs"))?;
    let mut params_file = std::fs::File::create(&Path::new(&out_dir).join("param.rs"))?;
    let mut oscquery_file = std::fs::File::create(&Path::new(&out_dir).join("oscquery.rs"))?;

    {
        let variants: Vec<DataType> = [
            ("Bool", "bool", "bool"),
            ("U8", "u8", "u8"),
            ("USize", "usize", "usize"),
            ("ISize", "isize", "isize"),
            ("Float", "float", "::sched::Float"),

            //complex types
            ("ClockData", "clock_data", "::sched::binding::bpm::ClockData"),
            ("TickResched", "tick_resched", "::sched::tick::TickResched"),
            ("TickSched", "tick_sched", "::sched::tick::TickSched"),
        ].iter().map(|d| DataType::new(d.0, d.1, d.2)).collect();

        let access: Vec<DataAccess> = [
            ("Get", "ParamBindingGet", true),
            ("Set", "ParamBindingSet", true),
            ("GetSet", "ParamBindingGetSet", false),
            ("KeyValueGet", "ParamBindingKeyValueGet", true),
            ("KeyValueSet", "ParamBindingKeyValueSet", true),
            ("KeyValueGetSet", "ParamBindingKeyValueGetSet", false),
        ].iter().map(|a| DataAccess::new(a.0, a.1, a.2)).collect();

        let mut froms = Vec::new();
        let mut data_type_name = Vec::new();
        let mut access_names = Vec::new();

        let mut param_typed_getters = Vec::new();
        
        for a in access.iter() {
            let ename = a.enum_name.clone();
            let tname = a.trait_name.clone();
            let access_var = a.access_var.clone();

            let mut entries = Vec::new();
            let access_name = a.access_name;
            access_names.push(quote! {
                Self::#access_var(..) => &#access_name
            });

            for v in variants.iter() {
                let n = v.var_name.clone();
                let t = v.typ.clone();

                //probably a better way to do this
                let inner = if a.dynamic {
                    quote! { Arc<dyn #tname<#t>> }
                } else {
                    quote! { Arc<#tname<#t>> }
                };
                entries.push(quote! { #n(#inner) });
                froms.push(quote! {
                    impl From<#inner> for ParamDataAccess {
                        fn from(param: #inner) -> Self {
                            Self::#access_var(#ename::#n(param))
                        }
                    }
                });
                let tname = format!("{}{}{}", a.type_name_prefix,  v.type_name.clone(), a.type_name_suffix);
                data_type_name.push(quote! {
                    Self::#access_var(#ename::#n(..)) => &#tname
                });
            }
            params_file.write_all(
                quote! {
                    pub enum #ename {
                        #(#entries),*
                    }
                }
                .to_string()
                .as_bytes(),
            )?;
        }

        for v in variants.iter() {
            let t = v.typ.clone();
            let ename = v.var_name.clone();

            param_typed_getters.push(quote! {
                impl AsParamGet<#t> for Param {
                    fn as_get(&self) -> Option<::std::sync::Arc<dyn ParamBindingGet<#t>>> {
                        match &self.data {
                            ParamDataAccess::Get(ParamDataGet::#ename(d)) => Some(d.clone()),
                            ParamDataAccess::GetSet(ParamDataGetSet::#ename(d)) => Some(d.clone() as _),
                            _ => None
                        }
                    }
                }
                impl AsParamSet<#t> for Param {
                    fn as_set(&self) -> Option<::std::sync::Arc<dyn ParamBindingSet<#t>>> {
                        match &self.data {
                            ParamDataAccess::Set(ParamDataSet::#ename(d)) => Some(d.clone()),
                            ParamDataAccess::GetSet(ParamDataGetSet::#ename(d)) => Some(d.clone() as _),
                            _ => None
                        }
                    }
                }
                impl AsParamKeyValueGet<#t> for Param {
                    fn as_key_value_get(&self) -> Option<::std::sync::Arc<dyn ParamBindingKeyValueGet<#t>>> {
                        match &self.data {
                            ParamDataAccess::KeyValueGet(ParamDataKeyValueGet::#ename(d)) => Some(d.clone()),
                            ParamDataAccess::KeyValueGetSet(ParamDataKeyValueGetSet::#ename(d)) => Some(d.clone() as _),
                            _ => None
                        }
                    }
                }
                impl AsParamKeyValueSet<#t> for Param {
                    fn as_key_value_set(&self) -> Option<::std::sync::Arc<dyn ParamBindingKeyValueSet<#t>>> {
                        match &self.data {
                            ParamDataAccess::KeyValueSet(ParamDataKeyValueSet::#ename(d)) => Some(d.clone()),
                            ParamDataAccess::KeyValueGetSet(ParamDataKeyValueGetSet::#ename(d)) => Some(d.clone() as _),
                            _ => None
                        }
                    }
                }
            });
        }

        params_file.write_all(
            quote! {
                impl ParamDataAccess {
                    pub fn data_type_name(&self) -> &'static str {
                        match self {
                            #(#data_type_name),*
                        }
                    }

                    pub fn access_name(&self) -> &'static str {
                        match self {
                            #(#access_names),*
                        }
                    }
                }
                #(#froms)*

                #(#param_typed_getters)*
            }
            .to_string()
            .as_bytes(),
        )?;
    }

    //(enum Varaiant Name, str for function naming, actual type name)
    let variants = [
        ("Bool", "bool", "bool"),
        ("U8", "u8", "u8"),
        ("USize", "usize", "usize"),
        ("ISize", "isize", "isize"),
        ("Float", "float", "Float"),

        //complex types
        ("ClockData", "clock_data", "ClockData"),
        ("TickResched", "tick_resched", "TickResched"),
        ("TickSched", "tick_sched", "TickSched"),
    ];

    //build out Get, Set, GetSet, ParamGet and ParamSet
    {
        let mut access_variants = Vec::new();
        let mut access_data_type_name = Vec::new();
        let mut access_access_name = Vec::new();
        let mut access_news = Vec::new();
        let mut access_froms = Vec::new();

        let mut pget = Vec::new();
        let mut pset = Vec::new();
        let mut param_data_type_name = Vec::new();
        let mut unbind = Vec::new();

        for v in variants.iter() {
            let i = format_ident!("{}", v.0);
            let t: syn::Type = syn::parse_str( v.2 ).unwrap();

            let g = format_ident!("{}Get", v.0);
            let s = format_ident!("{}Set", v.0);
            let gs = format_ident!("{}GetSet", v.0);

            let data_type = t.clone();
            let tname = v.2;

            {
                let gnew = format_ident!("new_{}_get", v.1);
                let gnew_doc = format!("Construct a new Get {} with the given binding.", tname);
                let gnew_init = format_ident!("new_{}_get_init", v.1);
                let gnew_init_doc = format!("Construct a new Get {} with the given binding, initializing the last value.", tname);
                let snew = format_ident!("new_{}_set", v.1);
                let snew_doc = format!("Construct a new Set {} with the given binding.", tname);
                let gsnew = format_ident!("new_{}_get_set", v.1);
                let gsnew_doc = format!("Construct a new GetSet {} with the given binding.", tname);
                let gsnew_init = format_ident!("new_{}_get_set_init", v.1);
                let gsnew_init_doc = format!("Construct a new GetSet {} with the given binding, initializing the last value.", tname);

                access_news.push(quote! {
                    #[doc = #gnew_doc]
                    pub fn #gnew<B: ParamBindingGet<#data_type> + 'static>(binding: B) -> Self {
                        Self::#g(Arc::new(BindingLastGet::new(binding)))
                    }
                    #[doc = #gnew_init_doc]
                    pub fn #gnew_init<B: ParamBindingGet<#data_type> + 'static>(binding: B) -> Self {
                        Self::#g(Arc::new(BindingLastGet::new_init(binding)))
                    }
                    #[doc = #snew_doc]
                    pub fn #snew<B: ParamBindingSet<#data_type> + 'static>(binding: B) -> Self {
                        Self::#s(Arc::new(BindingLastSet::new(binding)))
                    }
                    #[doc = #gsnew_doc]
                    pub fn #gsnew<B: ParamBinding<#data_type> + 'static>(binding: B) -> Self {
                        Self::#gs(Arc::new(BindingLastGetSet::new(binding)))
                    }
                    #[doc = #gsnew_init_doc]
                    pub fn #gsnew_init<B: ParamBinding<#data_type> + 'static>(binding: B) -> Self {
                        Self::#gs(Arc::new(BindingLastGetSet::new_init(binding)))
                    }
                });
                access_froms.push(quote! {
                    impl From<Arc<dyn ParamBindingGet<#data_type>>> for Access {
                        fn from(binding: Arc<dyn ParamBindingGet<#data_type>>) -> Self {
                            Self::#gnew_init(binding)
                        }
                    }
                    impl From<Arc<dyn ParamBindingSet<#data_type>>> for Access {
                        fn from(binding: Arc<dyn ParamBindingSet<#data_type>>) -> Self {
                            Self::#snew(binding)
                        }
                    }
                    impl From<Arc<dyn ParamBinding<#data_type>>> for Access {
                        fn from(binding: Arc<dyn ParamBinding<#data_type>>) -> Self {
                            Self::#gsnew_init(binding)
                        }
                    }
                });
            }

            //build the variants
            access_variants.push(quote! {
                #g(Arc<BindingLastGet<#t>>),
                #s(Arc<BindingLastSet<#t>>),
                #gs(Arc<BindingLastGetSet<#t>>)
            });

            //get the data type name
            access_data_type_name.push(quote! {
                Self::#g(..) | Self::#s(..) | Self::#gs(..) => &#tname
            });
            param_data_type_name.push(quote! {
                    Self::Get{ get : ParamGet::#i(..), .. } | Self::Set { set: ParamSet::#i(..), .. } => &#tname
                });
            //get the access name
            access_access_name.push(quote! {
                Self::#g(..) => &"get",
                Self::#s(..) => &"set",
                Self::#gs(..) => &"getset"
            });
            pget.push(quote! {
                #i(::std::sync::Arc<BindingSwapGet<#t>>)
            });
            pset.push(quote! {
                #i(::std::sync::Arc<BindingSwapSet<#t>>)
            });
            unbind.push(quote! {
                Self::#i(b) => {
                    b.unbind();
                }
            });
        }
        bindings_file.write_all(
            quote! {
                /// Operations/data and their access.
                pub enum Access {
                    #(#access_variants),*
                }
                impl Access {
                    pub fn access_name(&self) -> &'static str {
                        match self {
                            #(#access_access_name),*
                        }
                    }
                    pub fn data_type_name(&self) -> &'static str {
                        match self {
                            #(#access_data_type_name),*
                        }
                    }
                    #(#access_news)*
                }

                #(#access_froms)*
            }
            .to_string()
            .as_bytes(),
        )?;
        params_file.write_all(
            quote! {
                /// Parameters that you can get values from.
                pub enum ParamGet {
                    #(#pget),*
                }
                /// Parameters that you can set to a value.
                pub enum ParamSet {
                    #(#pset),*
                }
                impl ParamGet {
                    //TODO transform and return output?
                    pub fn unbind(&self) {
                        match self {
                            #(#unbind),*
                        }
                    }
                }
                impl ParamSet {
                    //TODO transform and return output?
                    pub fn unbind(&self) {
                        match self {
                            #(#unbind),*
                        }
                    }
                }
            }
            .to_string()
            .as_bytes(),
        )?;

        let mut param_type_name_get_variants = Vec::new();
        let mut param_type_name_set_variants = Vec::new();
        let mut binding_typed_getset = Vec::new();

        let mut try_bind_variants = Vec::new();
        for (var, fname, tname) in variants.iter() {
            let g = format_ident!("as_{}_get", fname);
            let s = format_ident!("as_{}_set", fname);
            let i = format_ident!("{}", var);
            try_bind_variants.push(quote! {
                ParamAccess::Get { get: ParamGet::#i(p), binding: b } => {
                    if let Some(g) = binding.#g() {
                        let mut l = b.lock();
                        p.bind(g);
                        l.replace(binding);
                        Ok(())
                    } else {
                        Err(BindingError::NoGet)
                    }
                }
                ParamAccess::Set { set: ParamSet::#i(p), binding: b } => {
                    if let Some(s) = binding.#s() {
                        let mut l = b.lock();
                        p.bind(s);
                        l.replace(binding);
                        Ok(())
                    } else {
                        Err(BindingError::NoSet)
                    }
                }
            });
            //build up get and set
            param_type_name_get_variants.push(quote! {
                ParamGet::#i(_) => #fname
            });
            param_type_name_set_variants.push(quote! {
                ParamSet::#i(_) => #fname
            });

            let get_ident = format_ident!("as_{}_get", fname);
            let set_ident = format_ident!("as_{}_set", fname);
            let t: syn::Type = syn::parse_str( tname ).unwrap();
            let g = format_ident!("{}Get", var);
            let s = format_ident!("{}Set", var);
            let gs = format_ident!("{}GetSet", var);

            binding_typed_getset.push(quote! {
                pub fn #get_ident(&self) -> Option<::std::sync::Arc<dyn ParamBindingGet<#t>>> {
                    match &self.binding {
                        Access::#g(m) => Some(m.clone()),
                        Access::#gs(m) => Some(m.clone() as _),
                        _ => None
                    }
                }
                pub fn #set_ident(&self) -> Option<std::sync::Arc<dyn ParamBindingSet<#t>>> {
                    match &self.binding {
                        Access::#s(m) => Some(m.clone()),
                        Access::#gs(m) => Some(m.clone() as _),
                        _ => None
                    }
                }
            });
        }

        bindings_file.write_all(
            quote! {
                impl Instance {
                    #(#binding_typed_getset)*
                }
            }
            .to_string()
            .as_bytes(),
        )?;

        params_file.write_all(
            quote! {
                impl ParamAccess {
                    fn binding(&self) -> &Mutex<Option<Arc<Instance>>> {
                        match self {
                            Self::Get { binding: b, .. } => b,
                            Self::Set { binding: b, .. } => b,
                        }
                    }

                    /// Get the uuid of the bound param, if there is a binding.
                    pub fn uuid(&self) -> Option<uuid::Uuid> {
                        self.binding().lock().as_deref().map(|b| b.uuid().clone())
                    }

                    pub fn data_type_name(&self) -> &'static str {
                        match self {
                        #(#param_data_type_name),*
                        }
                    }

                    /// attempt to bind.
                    pub fn try_bind(&self, binding: Arc<Instance>) -> Result<(), BindingError> {
                        let b = match self {
                            #(#try_bind_variants)*
                        };
                        b
                    }

                    ///Get a `&str` representing the type of access: `"get", "set" or "getset"`
                    pub fn access_name(&self) -> &str {
                        match self {
                            ParamAccess::Get{ .. } => "get",
                            ParamAccess::Set{ .. } => "set",
                        }
                    }
                }
            }
            .to_string()
            .as_bytes(),
        )?;
    }

    struct SimpBindingValue {
        pub bind_prefix: &'static str,
        pub osc_variant: &'static str,
        pub osc_type: &'static str,
        pub get_func: TokenStream,
        pub set_func: TokenStream,
        pub clip: Option<TokenStream>,
        pub range: Option<TokenStream>,
    }

    impl Default for SimpBindingValue {
        fn default() -> Self {
            Self {
                bind_prefix: &"",
                osc_variant: &"",
                osc_type: &"",
                get_func: quote! { unimplemented!(); },
                set_func: quote! { unimplemented!(); },
                clip: None,
                range: None,
            }
        }
    }

    {
        let simp = [
            SimpBindingValue {
                bind_prefix: &"Bool",
                osc_variant: &"Bool",
                osc_type: &"bool",
                get_func: quote! {
                    g.upgrade().map_or(false, |g| g.last().unwrap_or(false))
                },
                set_func: quote! {
                    s.upgrade().map(|s| s.set(v));
                },
                ..Default::default()
            },
            SimpBindingValue {
                bind_prefix: &"U8",
                osc_variant: &"Int",
                osc_type: &"i32",
                get_func: quote! {
                    g.upgrade().map_or(0, |g| g.last().unwrap_or(0) as i32)
                },
                set_func: quote! {
                    s.upgrade().map(|s| s.set(num::clamp(v, 0, 255) as u8));
                },
                clip: Some(quote! {
                    ClipMode::Both
                }),
                range: Some(quote! {
                    Range::MinMax(0, 255)
                }),
                ..Default::default()
            },
            SimpBindingValue {
                bind_prefix: &"USize",
                osc_variant: &"Long",
                osc_type: &"i64",
                get_func: quote! {
                    g.upgrade().map_or(0i64, |g| g.last().unwrap_or(0usize) as i64)
                },
                set_func: quote! {
                    s.upgrade().map(|s| s.set(std::cmp::max(v, 0i64) as usize));
                },
                clip: Some(quote! {
                    ClipMode::Low
                }),
                range: Some(quote! {
                    Range::Min(0)
                }),
                ..Default::default()
            },
            SimpBindingValue {
                bind_prefix: &"ISize",
                osc_variant: &"Long",
                osc_type: &"i64",
                get_func: quote! {
                    g.upgrade().map_or(0i64, |g| g.last().unwrap_or(0) as i64)
                },
                set_func: quote! {
                    s.upgrade().map(|s| s.set(v as isize));
                },
                ..Default::default()
            },
            SimpBindingValue {
                bind_prefix: &"Float",
                osc_variant: &"Double",
                osc_type: &"f64",
                get_func: quote! {
                    g.upgrade().map_or(0.0, |g| g.last().unwrap_or(0.0))
                },
                set_func: quote! {
                    s.upgrade().map(|s| s.set(v));
                },
                ..Default::default()
            },
        ];

        let mut access_values = Vec::new();
        for v in simp.iter() {
            let osc_variant = format_ident!("{}", v.osc_variant);
            let osc_type = format_ident!("{}", v.osc_type);
            let g = format_ident!("{}Get", v.bind_prefix);
            let s = format_ident!("{}Set", v.bind_prefix);
            let gs = format_ident!("{}GetSet", v.bind_prefix);
            let gf = v.get_func.clone();
            let sf = v.set_func.clone();
            let clip = v.clip.clone().unwrap_or(quote! { Default::default() });
            let range = v.range.clone().unwrap_or(quote! { Default::default() });
            access_values.push(quote! {
                Access::#g(g) => {
                    let g = Arc::downgrade(&g);
                    let _ = self.server.add_node(
                        oscquery::node::Get::new(
                            name,
                            description,
                            vec![ParamGet::#osc_variant(
                                ValueBuilder::new(Arc::new(GetFunc::new(move || {
                                    #gf
                                })) as _)
                                .with_clip_mode(#clip)
                                .with_range(#range)
                                .build(),
                            )],
                        )
                        .unwrap(),
                        Some(handle),
                    );
                    }
            });
            access_values.push(quote! {
                Access::#s(s) => {
                    let s = Arc::downgrade(&s);
                    let _ = self.server.add_node(
                        oscquery::node::Set::new(
                            name,
                            description,
                            vec![ParamSet::#osc_variant(
                                ValueBuilder::new(Arc::new(SetFunc::new(move |v: #osc_type| {
                                    #sf
                                })) as _)
                                .with_clip_mode(#clip)
                                .with_range(#range)
                                .build(),
                            )],
                            None
                        )
                        .unwrap(),
                        Some(handle),
                    );
                    }
            });
            access_values.push(quote! {
                Access::#gs(gs) => {
                    let g = Arc::downgrade(&gs);
                    let s = g.clone();
                    let _ = self.server.add_node(
                        oscquery::node::GetSet::new(
                            name,
                            description,
                            vec![ParamGetSet::#osc_variant(
                                ValueBuilder::new(Arc::new(GetSetFuncs::new(
                                    move || {
                                        #gf
                                    },
                                    move |v: #osc_type| {
                                        #sf
                                    },
                                )) as _)
                                .with_clip_mode(#clip)
                                .with_range(#range)
                                .build(),
                            )],
                            None,
                        )
                        .unwrap(),
                        Some(handle),
                    );
                }
            });
        }

        oscquery_file.write_all(
            quote! {
                impl OSCQueryHandler {
                    fn add_binding_value(&self, instance: &Arc<Instance>, handle: ::oscquery::root::NodeHandle) {
                        fn to_get<T>(weak: &Weak<dyn ::sched::binding::last::BindingLast<T>>) -> T 
                            where T: Default + Copy + Send + Sync
                        {
                            weak.upgrade().map_or(T::default(), |g| g.last().unwrap_or(T::default()))
                        }
                        let tick_resched_range = 
                            Range::Vals(vec![
                                "None",
                                "Relative",
                                "ContextRelative"
                            ].iter().map(|s| s.to_string().into()).collect());

                        let name = &"value";
                        let description: Option<&str> = Some(&"binding value");
                        match instance.binding() {
                            #(#access_values),*
                            Access::ClockDataGet(g) => {
                                let g = Arc::downgrade(&g) as Weak<dyn ::sched::binding::last::BindingLast<ClockData>>;
                                let bpmg = g.clone();
                                let ppqg = g.clone();
                                let microg = g.clone();
                                let _ = self.server.add_node(
                                    oscquery::node::Get::new(
                                        "value",
                                        Some("beats per minute, ppq, period micros"),
                                        vec![
                                        ParamGet::Double(
                                            ValueBuilder::new(Arc::new(GetFunc::new(move || {
                                                to_get::<ClockData>(&bpmg).bpm()
                                            })) as _)
                                            .with_clip_mode(ClipMode::Low)
                                            .with_range(Range::Min(0.0))
                                            .build(),
                                        ),
                                        ParamGet::Long(
                                            ValueBuilder::new(Arc::new(GetFunc::new(move || {
                                                to_get::<ClockData>(& ppqg).ppq() as i64
                                            })) as _)
                                            .with_clip_mode(ClipMode::Low)
                                            .with_range(Range::Min(1))
                                            .build(),
                                        ),
                                        ParamGet::Double(
                                            ValueBuilder::new(Arc::new(GetFunc::new(move || {
                                                to_get::<ClockData>(&microg).period_micros()
                                            })) as _)
                                            .with_clip_mode(ClipMode::Low)
                                            .with_range(Range::Min(0.0))
                                            .build(),
                                        ),
                                        ],
                                        )
                                            .unwrap(),
                                            Some(handle),
                                            ).unwrap();
                            },
                            Access::ClockDataSet(gs) => {
                                let s = Arc::downgrade(&gs) as Weak<dyn ParamBindingSet<ClockData>>;
                                let _ = self.server.add_node(
                                    oscquery::node::Set::new(
                                        "value",
                                        Some("beats per minute, ppq, period micros"),
                                        vec![
                                        ParamSet::Double(
                                            ValueBuilder::new(Arc::new(()) as _)
                                            .with_clip_mode(ClipMode::Low)
                                            .with_range(Range::Min(0.0))
                                            .build(),
                                        ),
                                        ParamSet::Long(
                                            ValueBuilder::new(Arc::new(()) as _)
                                            .with_clip_mode(ClipMode::Low)
                                            .with_range(Range::Min(1))
                                            .build(),
                                        ),
                                        ParamSet::Double(
                                            ValueBuilder::new(Arc::new(()) as _)
                                            .with_clip_mode(ClipMode::Low)
                                            .with_range(Range::Min(0.0))
                                            .build(),
                                        ),
                                        ],
                                        Some(
                                        Box::new(OscUpdateFunc::new(move |
                                            args: &Vec<oscquery::osc::OscType>,
                                            _addr: Option<SocketAddr>,
                                            _time: Option<(u32, u32)>,
                                            _handle: &NodeHandle,
                                            | -> Option<OscWriteCallback> {
                                            if let Some(s) = s.upgrade() {
                                                //update all the clock data parameters and then set
                                                let mut args = args.iter();
                                                let mut data: ClockData = Default::default();
                                                if let Some(::oscquery::osc::OscType::Double(v)) = args.next() {
                                                    data.set_bpm(0f64.max(*v));
                                                    if let Some(::oscquery::osc::OscType::Long(v)) = args.next() {
                                                        data.set_ppq(std::cmp::max(*v, 1) as usize);
                                                        if let Some(::oscquery::osc::OscType::Double(v)) = args.next() {
                                                            data.set_period_micros(0f64.max(*v));
                                                        }
                                                    }
                                                    s.set(data);
                                                }
                                            }
                                            None
                                        }))
                                        ))
                                            .unwrap(),
                                            Some(handle),
                                            ).unwrap();
                            },
                            Access::ClockDataGetSet(gs) => {
                                let s = Arc::downgrade(&gs) as Weak<dyn ParamBindingSet<ClockData>>;
                                let bl = Arc::downgrade(&gs) as Weak<dyn ::sched::binding::last::BindingLast<ClockData>>;
                                let bpmg = bl.clone();
                                let ppqg = bl.clone();
                                let microg = bl.clone();
                                let _ = self.server.add_node(
                                    oscquery::node::GetSet::new(
                                        "value",
                                        Some("beats per minute, ppq, period micros"),
                                        vec![
                                        ParamGetSet::Double(
                                            ValueBuilder::new(Arc::new(GetFunc::new(move || {
                                                to_get::<ClockData>(&bpmg).bpm()
                                            })) as _)
                                            .with_clip_mode(ClipMode::Low)
                                            .with_range(Range::Min(0.0))
                                            .build(),
                                        ),
                                        ParamGetSet::Long(
                                            ValueBuilder::new(Arc::new(GetFunc::new(move || {
                                                to_get::<ClockData>(& ppqg).ppq() as i64
                                            })) as _)
                                            .with_clip_mode(ClipMode::Low)
                                            .with_range(Range::Min(1))
                                            .build(),
                                        ),
                                        ParamGetSet::Double(
                                            ValueBuilder::new(Arc::new(GetFunc::new(move || {
                                                to_get::<ClockData>(&microg).period_micros()
                                            })) as _)
                                            .with_clip_mode(ClipMode::Low)
                                            .with_range(Range::Min(0.0))
                                            .build(),
                                        ),
                                        ],
                                        Some(
                                            Box::new(OscUpdateFunc::new(move |
                                                    args: &Vec<oscquery::osc::OscType>,
                                                    _addr: Option<SocketAddr>,
                                                    _time: Option<(u32, u32)>,
                                                    _handle: &NodeHandle,
                                                    | -> Option<OscWriteCallback> {
                                                        if let Some(s) = s.upgrade() {
                                                            //update all the clock data parameters and then set
                                                            let mut args = args.iter();
                                                            //by default, set to the last we had
                                                            let mut data: ClockData = to_get::<ClockData>(&bl);
                                                            if let Some(::oscquery::osc::OscType::Double(v)) = args.next() {
                                                                data.set_bpm(0f64.max(*v));
                                                                if let Some(::oscquery::osc::OscType::Long(v)) = args.next() {
                                                                    data.set_ppq(std::cmp::max(*v, 1) as usize);
                                                                    if let Some(::oscquery::osc::OscType::Double(v)) = args.next() {
                                                                        data.set_period_micros(0f64.max(*v));
                                                                    }
                                                                }
                                                                s.set(data);
                                                            }
                                                        }
                                                        None
                                                    }))
                                        ))
                                            .unwrap(),
                                            Some(handle),
                                            ).unwrap();
                            },
                            Access::TickReschedGet(g) => {
                                let g = Arc::downgrade(&g) as Weak<dyn ::sched::binding::last::BindingLast<TickResched>>;
                                let gt = g.clone();
                                let gv = g.clone();
                                let _ = self.server.add_node(
                                    oscquery::node::Get::new(
                                        "value",
                                        Some(&"variant, value"),
                                        vec![
                                        ParamGet::String(
                                            ValueBuilder::new(Arc::new(GetFunc::new(move || {
                                                match to_get::<TickResched>(&gt) {
                                                    TickResched::None => "None",
                                                    TickResched::Relative(..) => "Relative",
                                                    TickResched::ContextRelative(..) => "ContextRelative",
                                                }.to_string()
                                            })) as _)
                                            .with_range(tick_resched_range.clone())
                                            .build(),
                                        ),
                                        ParamGet::Long(
                                            ValueBuilder::new(Arc::new(GetFunc::new(move || {
                                                match to_get::<TickResched>(&gv) {
                                                    TickResched::None => 0,
                                                    TickResched::Relative(v) => v as i64,
                                                    TickResched::ContextRelative(v) => v as i64,
                                                }
                                            })) as _)
                                            .build(),
                                        )
                                        ],
                                        )
                                            .unwrap(),
                                            Some(handle),
                                            ).unwrap();
                            },
                            Access::TickReschedGetSet(gs) => {
                                let s = Arc::downgrade(&gs) as Weak<dyn ParamBindingSet<TickResched>>;
                                let g = Arc::downgrade(&gs) as Weak<dyn ::sched::binding::last::BindingLast<TickResched>>;
                                let gt = g.clone();
                                let gv = g.clone();
                                let _ = self.server.add_node(
                                    oscquery::node::GetSet::new(
                                        "value",
                                        Some(&"variant, value"),
                                        vec![
                                        ParamGetSet::String(
                                            ValueBuilder::new(Arc::new(GetFunc::new(move || {
                                                match to_get::<TickResched>(&gt) {
                                                    TickResched::None => "None",
                                                    TickResched::Relative(..) => "Relative",
                                                    TickResched::ContextRelative(..) => "ContextRelative",
                                                }.to_string()
                                            })) as _)
                                            .with_range(tick_resched_range.clone())
                                            .build(),
                                        ),
                                        ParamGetSet::Long(
                                            ValueBuilder::new(Arc::new(GetFunc::new(move || {
                                                match to_get::<TickResched>(&gv) {
                                                    TickResched::None => 0,
                                                    TickResched::Relative(v) => v as i64,
                                                    TickResched::ContextRelative(v) => v as i64,
                                                }
                                            })) as _)
                                            .build(),
                                        )
                                        ],
                                        Some(
                                            Box::new(OscUpdateFunc::new(move |
                                                    args: &Vec<oscquery::osc::OscType>,
                                                    _addr: Option<SocketAddr>,
                                                    _time: Option<(u32, u32)>,
                                                    _handle: &NodeHandle,
                                                    | -> Option<OscWriteCallback> {
                                                        if let Some(s) = s.upgrade() {
                                                            let mut args = args.iter();
                                                            if let Some(::oscquery::osc::OscType::String(v)) = args.next() {
                                                                let n = std::cmp::max(0, if let Some(::oscquery::osc::OscType::Long(n)) = args.next() {
                                                                    *n
                                                                } else {
                                                                    0
                                                                }) as usize;
                                                                let data = match v.as_str() {
                                                                    "Relative" => TickResched::Relative(n),
                                                                    "ContextRelative" => TickResched::ContextRelative(n),
                                                                    "None" => TickResched::None,
                                                                    _ => to_get::<TickResched>(&g)
                                                                };
                                                                s.set(data);
                                                            }
                                                        }
                                                        None
                                                    }))
                                        )
                                            ).unwrap(),
                                            Some(handle))
                                                .unwrap();
                            }
                            _ => ()
                        }
                    }
                }
            }
            .to_string()
            .as_bytes(),
        )?;
    }
    
    //instance factory
    {
        let mut entries = Vec::new();

        for v in variants.iter() {
            let data_type: syn::Type = syn::parse_str( v.2 ).unwrap();
            let tname = v.2;

            let cname = format!("const::<{}>", tname);
            let cdesc = format!("Constant {} value", tname);

            let mname = format!("val::<{}>", tname);
            let mdesc = format!("Mutable {} value", tname);

            //consts and values
            entries.push(
                quote! {
                    let ex: #data_type = Default::default();

                    let f: Box<InstDataFn> = Box::new(|arg| {
                        let v: Result<#data_type, _> = serde_json::from_value(arg);
                        if let Ok(v) = v {
                            Ok(
                                (
                                    (Arc::new(v) as Arc<dyn ParamBindingGet<#data_type>>).into(),
                                    Default::default()
                                )
                            )
                        } else {
                            Err(CreateError::InvalidArgs)
                        }
                    });
                    m.insert(#cname, 
                        InstFactItem::new(f, #cdesc, Some(serde_json::to_string(&ex).unwrap())
                    ));
                    let f: Box<InstDataFn> = Box::new(|arg| {
                        let v: Result<#data_type, _> = serde_json::from_value(arg);
                        if let Ok(v) = v {
                            Ok(
                                (
                                    (Arc::new(Atomic::new(v)) as Arc<dyn ParamBinding<#data_type>>).into(),
                                    Default::default()
                                )
                            )
                        } else {
                            Err(CreateError::InvalidArgs)
                        }
                    });
                    m.insert(#mname, 
                        InstFactItem::new(f, #mdesc, Some(serde_json::to_string(&ex).unwrap())
                    ));
                }
            );
        }

        instance_factory_file.write_all(
            quote! {
                lazy_static::lazy_static! {
                    static ref INSTANCE_FACTORY_HASH: HashMap<&'static str, InstFactItem> = {
                        let mut m = HashMap::new();
                        #(#entries)*
                        m
                    };
                }
            }
            .to_string()
            .as_bytes()
        )?;
    }

    println!("cargo:rerun-if-changed=build.rs");
    Ok(())
}

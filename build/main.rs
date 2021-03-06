use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::env;
use std::io::Write;
use std::path::Path;

struct DataType {
    pub var_name: syn::Ident,
    pub func_name: &'static str,
    pub typ: syn::Type,
    pub type_name: String,
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

    let mut instance_factory_file = std::fs::File::create(&Path::new(&out_dir).join("instance_factory.rs"))?;
    let mut params_file = std::fs::File::create(&Path::new(&out_dir).join("param.rs"))?;
    let mut oscquery_file = std::fs::File::create(&Path::new(&out_dir).join("oscquery.rs"))?;

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

    //build out Param
    {
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
                impl std::convert::TryInto<::std::sync::Arc<dyn ParamBindingGet<#t>>> for &Param {
                    type Error = ();
                    fn try_into(self) -> Result<::std::sync::Arc<dyn ParamBindingGet<#t>>, ()> {
                        match &self.data {
                            ParamDataAccess::Get(ParamDataGet::#ename(d)) => Ok(d.clone()),
                            ParamDataAccess::GetSet(ParamDataGetSet::#ename(d)) => Ok(d.clone() as _),
                            _ => Err(())
                        }
                    }
                }
                impl std::convert::TryInto<::std::sync::Arc<dyn ParamBindingSet<#t>>> for &Param {
                    type Error = ();
                    fn try_into(self) -> Result<::std::sync::Arc<dyn ParamBindingSet<#t>>, ()> {
                        match &self.data {
                            ParamDataAccess::Set(ParamDataSet::#ename(d)) => Ok(d.clone()),
                            ParamDataAccess::GetSet(ParamDataGetSet::#ename(d)) => Ok(d.clone() as _),
                            _ => Err(())
                        }
                    }
                }
                impl std::convert::TryInto<::std::sync::Arc<dyn ParamBindingKeyValueGet<#t>>> for &Param {
                    type Error = ();
                    fn try_into(self) -> Result<::std::sync::Arc<dyn ParamBindingKeyValueGet<#t>>, ()> {
                        match &self.data {
                            ParamDataAccess::KeyValueGet(ParamDataKeyValueGet::#ename(d)) => Ok(d.clone()),
                            ParamDataAccess::KeyValueGetSet(ParamDataKeyValueGetSet::#ename(d)) => Ok(d.clone() as _),
                            _ => Err(())
                        }
                    }
                }
                impl std::convert::TryInto<::std::sync::Arc<dyn ParamBindingKeyValueSet<#t>>> for &Param {
                    type Error = ();
                    fn try_into(self) -> Result<::std::sync::Arc<dyn ParamBindingKeyValueSet<#t>>, ()> {
                        match &self.data {
                            ParamDataAccess::KeyValueSet(ParamDataKeyValueSet::#ename(d)) => Ok(d.clone()),
                            ParamDataAccess::KeyValueGetSet(ParamDataKeyValueGetSet::#ename(d)) => Ok(d.clone() as _),
                            _ => Err(())
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

    //build out ParamGet and ParamSet
    {
        let mut pget = Vec::new();
        let mut pset = Vec::new();
        let mut param_data_type_name = Vec::new();
        let mut unbind = Vec::new();

        for v in variants.iter() {
            let i = v.var_name.clone();
            let t = v.typ.clone();
            let tname = v.type_name.clone();

            param_data_type_name.push(quote! {
                    Self::Get{ get : ParamGet::#i(..), .. } | Self::Set { set: ParamSet::#i(..), .. } => &#tname
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
        //for (var, fname, tname) in variants.iter() {
        for v in variants.iter() {
            let i = v.var_name.clone();
            let fname = v.func_name.clone();
            let t = v.typ.clone();

            try_bind_variants.push(quote! {
                ParamAccess::Get { get: ParamGet::#i(p), binding: b } => {
                    if let Ok(g) = binding.as_ref().try_into() {
                        let mut l = b.lock();
                        p.bind(g);
                        l.replace(binding);
                        Ok(())
                    } else {
                        Err(BindingError::NoGet)
                    }
                }
                ParamAccess::Set { set: ParamSet::#i(p), binding: b } => {
                    if let Ok(s) = binding.as_ref().try_into() {
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
            let g = format_ident!("{}Get", i);
            let s = format_ident!("{}Set", i);
            let gs = format_ident!("{}GetSet", i);

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

        params_file.write_all(
            quote! {
                impl ParamAccess {
                    fn binding(&self) -> &Mutex<Option<Arc<Param>>> {
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
                    pub fn try_bind(&self, binding: Arc<Param>) -> Result<(), BindingError> {
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
        pub variant_name: &'static str,
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
                variant_name: &"",
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
                variant_name: &"Bool",
                osc_variant: &"Bool",
                osc_type: &"bool",
                get_func: quote! {
                    g.upgrade().map_or(false, |g| g.get())
                },
                set_func: quote! {
                    s.upgrade().map(|s| s.set(v));
                },
                ..Default::default()
            },
            SimpBindingValue {
                variant_name: &"U8",
                osc_variant: &"Int",
                osc_type: &"i32",
                get_func: quote! {
                    g.upgrade().map_or(0, |g| g.get() as i32)
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
                variant_name: &"USize",
                osc_variant: &"Long",
                osc_type: &"i64",
                get_func: quote! {
                    g.upgrade().map_or(0i64, |g| g.get() as i64)
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
                variant_name: &"ISize",
                osc_variant: &"Long",
                osc_type: &"i64",
                get_func: quote! {
                    g.upgrade().map_or(0i64, |g| g.get() as i64)
                },
                set_func: quote! {
                    s.upgrade().map(|s| s.set(v as isize));
                },
                ..Default::default()
            },
            SimpBindingValue {
                variant_name: &"Float",
                osc_variant: &"Double",
                osc_type: &"f64",
                get_func: quote! {
                    g.upgrade().map_or(0.0, |g| g.get())
                },
                set_func: quote! {
                    s.upgrade().map(|s| s.set(v));
                },
                ..Default::default()
            },
        ];

        let mut access_values = Vec::new();
        for v in simp.iter() {
            let variant_name = format_ident!("{}", v.variant_name);
            let osc_variant = format_ident!("{}", v.osc_variant);
            let osc_type = format_ident!("{}", v.osc_type);
            let gf = v.get_func.clone();
            let sf = v.set_func.clone();
            let clip = v.clip.clone().unwrap_or(quote! { Default::default() });
            let range = v.range.clone().unwrap_or(quote! { Default::default() });
            access_values.push(quote! {
                crate::param::ParamDataAccess::Get(crate::param::ParamDataGet::#variant_name(g)) => {
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
                crate::param::ParamDataAccess::Set(crate::param::ParamDataSet::#variant_name(s)) => {
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
                crate::param::ParamDataAccess::GetSet(crate::param::ParamDataGetSet::#variant_name(gs)) => {
                    let g = Arc::downgrade(&gs) as Weak<dyn ::sched::binding::ParamBindingGet<_>>;
                    let s = Arc::downgrade(&gs) as Weak<dyn ::sched::binding::ParamBindingSet<_>>;
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
                    fn add_param_value(&self, shadow: &ParamDataAccess, handle: ::oscquery::root::NodeHandle) {

                        fn to_get<T>(weak: &Weak<dyn ::sched::binding::ParamBindingGet<T>>) -> T 
                            where T: Default + Copy + Send + Sync
                        {
                            weak.upgrade().map_or(T::default(), |g| g.get())
                        }

                        let tick_resched_range = 
                            Range::Vals(vec![
                                "None",
                                "Relative",
                                "ContextRelative"
                            ].iter().map(|s| s.to_string().into()).collect());

                        let name = &"value";
                        let description: Option<&str> = Some(&"binding value");
                        match shadow {
                            #(#access_values),*
                            crate::param::ParamDataAccess::Get(crate::param::ParamDataGet::ClockData(g)) => {
                                let g = Arc::downgrade(&g) as Weak<dyn ::sched::binding::ParamBindingGet<ClockData>>;
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
                            crate::param::ParamDataAccess::Set(crate::param::ParamDataSet::ClockData(gs)) => {
                                let s = Arc::downgrade(&gs) as Weak<dyn ::sched::binding::ParamBindingSet<ClockData>>;
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
                            crate::param::ParamDataAccess::GetSet(crate::param::ParamDataGetSet::ClockData(gs)) => {
                                let s = Arc::downgrade(&gs) as Weak<dyn ::sched::binding::ParamBindingSet<ClockData>>;
                                let g = Arc::downgrade(&gs) as Weak<dyn ::sched::binding::ParamBindingGet<ClockData>>;
                                let bpmg = g.clone();
                                let ppqg = g.clone();
                                let microg = g.clone();
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
                                                            let mut data: ClockData = to_get::<ClockData>(&g);
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
                            crate::param::ParamDataAccess::Get(crate::param::ParamDataGet::TickResched(g)) => {
                                let g = Arc::downgrade(&g) as Weak<dyn ::sched::binding::ParamBindingGet<TickResched>>;
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
                            crate::param::ParamDataAccess::GetSet(crate::param::ParamDataGetSet::TickResched(gs)) => {
                                let s = Arc::downgrade(&gs) as Weak<dyn ::sched::binding::ParamBindingSet<TickResched>>;
                                let g = Arc::downgrade(&gs) as Weak<dyn ::sched::binding::ParamBindingGet<TickResched>>;
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
            let data_type = v.typ.clone();
            let tname = v.type_name.clone();

            let cname = format!("const::<{}>", tname);
            let cdesc = format!("Constant {} value", tname);

            let mname = format!("val::<{}>", tname);
            let mdesc = format!("Mutable {} value", tname);

            //consts and values
            entries.push(
                quote! {
                    let ex: #data_type = Default::default();

                    //constant
                    let f: Box<ParamDataFn> = Box::new(|arg| {
                        let v: Result<#data_type, _> = serde_json::from_value(arg);
                        if let Ok(v) = v {
                            let g = Arc::new(v) as Arc<dyn ParamBindingGet<#data_type>>;
                            Ok(
                                (
                                    g.clone().into(),
                                    Some(g.into()), //shadow for a const is just the same access
                                    Default::default()
                                )
                            )
                        } else {
                            Err(CreateError::InvalidArgs)
                        }
                    });
                    m.insert(#cname, 
                        ParamFactItem::new(f, #cdesc, Some(serde_json::to_string(&ex).unwrap())
                    ));

                    //value
                    let f: Box<ParamDataFn> = Box::new(|arg| {
                        let v: Result<#data_type, _> = serde_json::from_value(arg);
                        if let Ok(v) = v {
                            let gs = Arc::new(::sched::binding::ParamBindingGetSet::new(Arc::new(Atomic::new(v)) as Arc<dyn ParamBinding<#data_type>>));
                            Ok(
                                (
                                    gs.clone().into(),
                                    Some(gs.into()), //TODO shadow access should be queued
                                    Default::default()
                                )
                            )
                        } else {
                            Err(CreateError::InvalidArgs)
                        }
                    });
                    m.insert(#mname, 
                        ParamFactItem::new(f, #mdesc, Some(serde_json::to_string(&ex).unwrap())
                    ));
                }
            );
        }

        instance_factory_file.write_all(
            quote! {
                lazy_static::lazy_static! {
                    static ref PARAM_FACTORY_HASH: HashMap<&'static str, ParamFactItem> = {
                        let mut m = HashMap::new();
                        #(#entries)* m
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

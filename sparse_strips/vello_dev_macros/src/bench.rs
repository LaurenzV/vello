use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::quote;
use syn::{ItemFn, parse_macro_input};

pub(crate) fn vello_bench_inner(_: TokenStream, item: TokenStream) -> TokenStream {
    let mut input_fn = parse_macro_input!(item as ItemFn);

    let input_fn_name = input_fn.sig.ident.clone();
    let input_fn_name_str = input_fn.sig.ident.to_string();
    let inner_fn_name = Ident::new(&format!("{}_inner", input_fn_name), input_fn_name.span());

    input_fn.sig.ident = inner_fn_name.clone();

    let expanded = quote! {
        #input_fn

        pub fn #input_fn_name(c: &mut criterion::Criterion) {
            use vello_cpu::fine2::Fine;
            use vello_simd::{neon, fallback, Simd};
            use vello_common::coarse::WideTile;
            use vello_common::tile::Tile;

            fn get_bench_name(suffix1: &str, suffix2: &str) -> String {
                let module_path = module_path!();

                let module = module_path
                    .split("::")
                    .skip(1)
                    .collect::<Vec<_>>()
                    .join("/");

                format!("{}/{}_{}", module, suffix1, suffix2)
            }

            fn run_integer<S: Simd>(b: &mut Bencher, s: S) {
                let mut fine = Fine::<S::Integer>::new(WideTile::WIDTH, Tile::HEIGHT);
                #inner_fn_name(b, &mut fine);
            }

            fn run_float<S: Simd>(b: &mut Bencher, s: S) {
                let mut fine = Fine::<S::Float>::new(WideTile::WIDTH, Tile::HEIGHT);
                #inner_fn_name(b, &mut fine);
            }

            c.bench_function(&get_bench_name(&#input_fn_name_str, "u8"), |b| {
                run_integer(b, fallback::Fallback);
            });

            c.bench_function(&get_bench_name(&#input_fn_name_str, "f32"), |b| {
                run_float(b, fallback::Fallback);
            });

            c.bench_function(&get_bench_name(&#input_fn_name_str, "u8_neon"), |b| {
                let neon = neon::Neon::new().unwrap().get();
                run_integer(b, neon);
            });

            c.bench_function(&get_bench_name(&#input_fn_name_str, "f32_neon"), |b| {
                let neon = neon::Neon::new().unwrap().get();
                run_float(b, neon);
            });
        }
    };

    expanded.into()
}

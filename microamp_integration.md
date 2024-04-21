# uAMP integration into RTIC
In order to provide an intial support for multicore RTIC applications that compile to more than one binary (one binary per-core), uAMP framework is integrated into the current implementation. The integration only required minimal changes to the `rtic-core` crate, and the other compilation pass crates. And furthermore, this new functionality is guarded behind a rust feature: `multibin`, which can be enabled/disabled by the specific RTIC distribution (disabled by default), making this change fully transparent to RTIC distributions where only a single binary needs to be produced (application can be normally compiled with cargo-build without the need for cargo-microamp). And in the following we will provide more details on how this integration has been introduced.


First, the following feature is now declared in the `Cargo.toml` file of the `rtic-core` crate. 

```toml
# Cargo.toml
# ...
[features]
multibin = []
```

In addition, the following functions were added to `rtic-core` and are exported so that other compilation pass crates can use them

```rust
pub mod multibin {
    use syn::Attribute;

    /// If `multibin` feature is enabled, this returns a tokenstream for the attribute `#[cfg(core = "x")]` to partition an application
    /// to multiple binaries. Otherwise `None` is returned
    pub fn multibin_cfg_core(core: u32) -> Option<Attribute> {
        #[cfg(feature = "multibin")]
        {
            let val = core.to_string();
            Some(parse_quote! {
                #[cfg(core = #val)]
            })
        }
        #[cfg(not(feature = "multibin"))]
        None
    }

    /// If `multibin` feature is enabled, this returns a tokenstream for the attribute `#[multibin_shared]` to make sure the annotated variable is present at the same address on all cores. Otherwise `None` is returned
    pub fn multibin_shared() -> Option<Attribute> {
        #[cfg(feature = "multibin")]
        {
            Some(parse_quote! {
                #[multibin_shared]
            })
        }
        #[cfg(not(feature = "multibin"))]
        None
    }
}
```

The `multibin_cfg_core(core: u32)` function produces the attribute `#[cfg(core = "x")]` and the `multibin_shared()` function produces the attribute `#[multibin_shared]` where `multibin_shared` is will be an alias for `microamp::shared`. This introduced an additional `StandardPassImpl` Trait function that is required to be implemented when the `multibin` rust feature is enabled. The output of this function will be used in the code generation phase of the standard pass to produce the "use" statement for importing `microamp::shared` under the alias `multibin_shared`. 

```rust
pub trait StandardPassImpl {
    
    // ...
    
    /// Provide the path to the rexported microamp::shared attribute
    /// Example implementation can be 
    /// ```rust
    /// fn multibin_shared_crate_path() -> syn::Path {
    ///     syn::parse_quote! {
    ///         rtic::exports::microamp::shared
    ///     }
    /// }
    /// ```
    /// 
    /// This will be used in code generation to produce the use statement:
    /// ```rust
    /// use rtic::exports::microamp::shared as multibin_shared;
    /// ```
    #[cfg(feature = "multibin")]
    fn multibin_shared_path() -> syn::Path;
}
```

## Identifying where to generate #[multibin_shared] and #[cfg(core = "x")]
With the previous changes in place, the only thing remaining was to identify where the attributes `#[multibin_shared]` and `#[cfg(core = "x")]` needed to be placed in the code generation part, and the functions `multibin_shared()` and `multibin_cfg_core(core: u32)` were used to generate those attributes as follows:

```rust
let cfg_core = multibin_cfg_core(1);
quote!{
    #cfg_core
    let x = 5;
}
```

The previous code can generate two cases:  
1. the feature `multibin` is enabled. generates:
```rust
#[cfg(core = "1")]
let x = 5;
```

2. the feature `multibin` is NOT enabled. generates:
```rust
let x = 5;
```

## Conventions that other RTIC compilation-pass need to follow to support multi-binary RTIC
Some compilation pass crates like `rtic-sw-pass` (Software pass crate) generate parts of code which need to be annotated with #[cfg(core = "x")] or #[multibin_shared]. If that is the case, then:


1- the compilation pass crate must add `rtic-core` as a dependecy, to be able to use `multibin_shared()` and `multibin_cfg_core(core: u32)` functions.
2- The crate needs to define and propagate the `multibin` feature as follows
```toml
# Cargo.toml
# ...

[features]
multibin = ["rtic-core/multibin"]
```
3- in code generations, always identify the parts where the attribtes #[cfg(core = "x")], or #[multibin_shared] need to be used, then `multibin_shared()` and `multibin_cfg_core(core: u32)` based on that need.

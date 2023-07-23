#![cfg(test)]
use rkyv_typename::TypeName;

fn check_typename<T: TypeName + ?Sized>(type_name_check: &str) {
    let mut type_name_built = String::new();
    T::build_type_name(|piece| type_name_built += piece);
    assert_eq!(type_name_built, type_name_check)
}

// Test that the TypeName derive works for the most trivial cases

#[test]
fn builtin_types() {
    check_typename::<i32>("i32");
    check_typename::<(i32,)>("(i32,)");
    check_typename::<(i32, i32)>("(i32, i32)");
    check_typename::<[[u8; 4]; 8]>("[[u8; 4]; 8]");
    check_typename::<Option<[String; 1]>>("core::option::Option<[alloc::string::String; 1]>");
    check_typename::<Option<[Option<u8>; 4]>>(
        "core::option::Option<[core::option::Option<u8>; 4]>"
    );
}

#[test]
fn derive() {
    #[derive(TypeName)]
    struct Test;

    check_typename::<Test>("test_derive::Test");
}

#[test]
fn derive_generic() {
    #[derive(TypeName)]
    struct Test<T, U, V>(T, U, V);

    check_typename::<Test<u8, [i32; 4], Option<String>>>(
        "test_derive::Test<u8, [i32; 4], core::option::Option<alloc::string::String>>"
    );
}

#[test]
fn derive_custom_typename() {
    #[derive(TypeName)]
    #[typename = "Custom"]
    struct Test;

    check_typename::<Test>("Custom");

    #[derive(TypeName)]
    #[typename = "GenericCustom"]
    struct GenericTest<T>(T);

    check_typename::<GenericTest<i32>>("GenericCustom<i32>");
    check_typename::<GenericTest<Test>>("GenericCustom<Custom>");
}


#[test]
fn trivial_struct() {
    #[derive(TypeName)]
    struct Struct {}
    check_typename::<Struct>("test_derive::Struct");
}

#[test]
fn trivial_unit_struct() {
    #[derive(TypeName)]
    struct Struct;
    check_typename::<Struct>("test_derive::Struct");
}

#[test]
fn trivial_tuple_struct() {
    #[derive(TypeName)]
    struct Struct();
    check_typename::<Struct>("test_derive::Struct");
}

#[test]
fn trivial_enum() {
    #[derive(TypeName)]
    enum Enum {}
    check_typename::<Enum>("test_derive::Enum");
}

// Test non generic structs and enums

#[test]
#[allow(dead_code)]
fn basic_struct() {
    #[derive(TypeName)]
    struct Struct {
        a: u32,
        b: u128,
    }
    check_typename::<Struct>("test_derive::Struct");
}

#[test]
#[allow(dead_code)]
fn basic_enum() {
    #[derive(TypeName)]
    enum Enum {
        CaseA(u32),
        CaseB(u128),
    }
    check_typename::<Enum>("test_derive::Enum");
}

// Test basic generic structs

#[test]
#[allow(dead_code)]
fn generic_one_param() {
    #[derive(TypeName)]
    struct Struct<C> {
        a: u32,
        b: u128,
        c: C,
    }
    check_typename::<Struct<()>>("test_derive::Struct<()>");
}

#[test]
#[allow(dead_code)]
fn generic_name_collisions() {
    // The only difference with generic_one_param() is the type parameter F
    // which earlier caused a name collision with the generic parameter in
    // build_type_name<F: FnMut(&str)>(...) method.
    #[derive(TypeName)]
    struct Struct<F> {
        a: u32,
        b: u128,
        c: F,
    }
    check_typename::<Struct<()>>("test_derive::Struct<()>");
}

#[test]
#[allow(dead_code)]
fn generic_two_params() {
    #[derive(TypeName)]
    // More than one type parameter
    struct Struct<C, D> {
        a: u32,
        b: u128,
        c: (C, D),
    }
    check_typename::<Struct<(), u8>>("test_derive::Struct<(), u8>");
}

// Test that trait bounds are generated correctly

#[test]
#[allow(dead_code)]
fn generic_with_trait_bounds() {
    #[derive(TypeName)]
    struct Struct<C: std::fmt::Debug + Clone, D>
    where
        D: std::fmt::Display + Copy,
    {
        a: u32,
        b: u128,
        c: (C, D),
    }
    check_typename::<Struct<(), u8>>("test_derive::Struct<(), u8>");
}

// Test types with lifetimes

#[test]
#[allow(dead_code)]
fn lifetimes() {
    #[derive(TypeName)]
    struct Struct<'b> {
        a: u32,
        b: &'b u128,
    }
    check_typename::<Struct>("test_derive::Struct");
}

#[test]
#[allow(dead_code)]
fn two_lifetimes_with_bounds() {
    #[derive(TypeName)]
    struct Struct<'b, 'c>
    where
        'b: 'c,
    {
        a: u32,
        b: &'b u128,
        c: &'c i128,
    }
    check_typename::<Struct>("test_derive::Struct");
}

#[test]
#[allow(dead_code)]
fn combined_generic_type_and_lifetime() {
    #[derive(TypeName)]
    struct Struct<'a, C> {
        a: u32,
        b: &'a u128,
        c: C,
    }
    check_typename::<Struct<()>>("test_derive::Struct<()>");
}

// Test const generics

#[test]
#[allow(dead_code)]
fn const_generic_num() {
    #[derive(TypeName)]
    struct Struct<const N: usize> {
        a: u32,
        b: u128,
    }
    check_typename::<Struct<77>>("test_derive::Struct<77>");
}

#[test]
#[allow(dead_code)]
fn const_generic_bool() {
    #[derive(TypeName)]
    struct Struct<const T: bool> {
        a: u32,
        b: u128,
    }
    check_typename::<Struct<true>>("test_derive::Struct<true>");
}

#[test]
#[allow(dead_code)]
fn const_generic_char() {
    #[derive(TypeName)]
    struct Struct<const N: char> {
        a: u32,
        b: u128,
    }
    check_typename::<Struct<'a'>>("test_derive::Struct<'a'>");
}

// Test more composite cases which could potentially cause problems such as:
// - Nested types
// - Compex trait bounds
// - Combination of lifetimes, generic types and const generics

#[test]
#[allow(dead_code)]
fn composite_cases() {
    #[derive(TypeName)]
    struct Sub<'a, D, const N: usize>(u32, &'a u128, D)
    where
        D: 'a;

    #[derive(TypeName)]
    enum Enum<'a, D, const N: usize>
    where
        D: std::fmt::Debug + ?Sized,
    {
        Variant1(&'a D),
        Variant2(Sub<'a, &'a D, 77>),
    }

    #[derive(TypeName)]
    struct Struct<'a, C: std::fmt::Debug, D: Clone, E, F, G: ?Sized, const N: usize, const B: bool>
    where
        'a: 'static,
        C: Clone + 'a,
        D: 'static + ?Sized,
    {
        a: u32,
        b: &'a u128,
        c: C,
        d: Sub<'a, D, 44>,
        e: E,
        f: F,
        g: G,
    }
    let mod_path = module_path!();
    check_typename::<
        Struct<String, [[String; 3]; 2], Vec<Box<i128>>, &(&Enum<(u8, i8), 99>,), [u16], 33, false>,
    >(
        format!(
            "test_derive::Struct<\
                alloc::string::String, \
                [[alloc::string::String; 3]; 2], \
                alloc::vec::Vec<alloc::boxed::Box<i128>>, \
                &(&{mod_path}::Enum<(u8, i8), 99>,), \
                [u16], \
                33, \
                false\
        >")
        .as_str(),
    );
}

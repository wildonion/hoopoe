




/* 
    placing a file named build.rs in the root of a package will cause Cargo to 
    compile that script and execute it just before building the whole package.
*/
fn main() -> Result<(), Box<dyn std::error::Error>>{

    /* 
        this will build all proto files and convert them into
        Rust codes before compiling the Rust codes so we can 
        import them in our crates and build rpc servers and clients
    */
    tonic_build::compile_protos("proto/event.proto").unwrap();

    Ok(())
    
}
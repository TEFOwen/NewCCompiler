use std::{
    fs::{File, remove_file},
    io::BufReader,
    path::PathBuf,
    process::Command,
};

use clap::{Args, Parser};

#[derive(Parser)]
#[command(name = "compiler_driver")]
#[command(about = "A simple compiler driver", long_about = None)]
struct Cli {
    #[clap(help = "Input C source file(s)", required = true)]
    input: Vec<PathBuf>,

    #[command(flatten)]
    compile_options: CompileOptions,
}

#[derive(Args)]
#[group(required = false, multiple = false)]
struct CompileOptions {
    #[clap(long, action, help = "Run only the lexer")]
    lex: bool,

    #[clap(long, action, help = "Run only the lexer and parser")]
    parse: bool,

    #[clap(
        long,
        action,
        help = "Run only semantic analysis (no TACKY IR generated)"
    )]
    validate: bool,

    #[clap(
        long,
        action,
        help = "Run only TACKY intermediate representation generation"
    )]
    tacky: bool,

    #[clap(
        long,
        action,
        help = "Run only the lexer, parser, and assembly generation (no assembly written to file)"
    )]
    codegen: bool,

    #[clap(
        short = 'S',
        action,
        help = "Only generate assembly, do not assemble or link"
    )]
    assemble: bool,

    #[clap(
        short = 'c',
        action,
        help = "Only compile and assemble, do not link (no executable generated)"
    )]
    nolink: bool,
}

fn compile_file(
    input_path: &PathBuf,
    compile_options: &CompileOptions,
) -> Result<Option<PathBuf>, Box<dyn std::error::Error>> {
    let preprocessed_path = input_path.with_extension("i");
    assert!(
        Command::new("gcc")
            .args(&[
                "-E",
                "-P",
                input_path.to_str().unwrap(),
                "-o",
                preprocessed_path.to_str().unwrap(),
            ])
            .status()?
            .success()
    );

    let preprocessed_file = BufReader::new(File::open(&preprocessed_path)?);

    let tokens = match compiler::lexer::lex(preprocessed_file) {
        Ok(tokens) => tokens,
        Err(e) => {
            eprintln!("Lexing error: {}", e);
            if let Err(e) = remove_file(preprocessed_path) {
                eprintln!("Warning: failed to remove preprocessed file: {}", e);
            }
            std::process::exit(1);
        }
    };

    if let Err(e) = remove_file(preprocessed_path) {
        eprintln!("Warning: failed to remove preprocessed file: {}", e);
    }

    if compile_options.lex {
        print!("Tokens: [ ");
        for compiler::lexer::Token(token_type, _) in &tokens {
            print!("{:?} ", token_type);
        }
        println!("]");
        return Ok(None);
    }

    let ast = match compiler::parser::parse(tokens) {
        Ok(ast) => ast,
        Err(e) => {
            eprintln!("Parsing error: {}", e);
            std::process::exit(1);
        }
    };

    if compile_options.parse {
        println!("AST: {:#?}", ast);
        return Ok(None);
    }

    let resolved_ast = match compiler::semantic_analysis::resolve_program(ast) {
        Ok(resolved_ast) => {
            if compile_options.validate {
                println!("Semantic analysis successful: {:#?}", resolved_ast);
                return Ok(None);
            }
            resolved_ast
        }
        Err(e) => {
            eprintln!("Semantic analysis error: {}", e);
            std::process::exit(1);
        }
    };

    let tacky = compiler::tacky::ToTacky::to_tacky(resolved_ast);

    if compile_options.tacky {
        println!("TACKY IR: {:#?}", tacky);
        return Ok(None);
    }

    let assembly = compiler::codegen::to_assembly(tacky);

    if compile_options.codegen {
        println!("Assembly: {:#?}", assembly);
        return Ok(None);
    }

    let assembly_path = input_path.with_extension("s");
    let assembly_file = File::create(&assembly_path)?;
    compiler::codeemission::EmitCode::emit_code(&assembly, assembly_file)?;

    if compile_options.assemble {
        println!("Assembly written to: {}", assembly_path.to_str().unwrap());
    }

    Ok(Some(assembly_path))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();

    if args.input.len() == 0 {
        eprintln!("Error: no input files provided");
        std::process::exit(1);
    }

    let mut assembly_paths = Vec::new();
    for input_path in &args.input {
        match compile_file(input_path, &args.compile_options) {
            Err(e) => {
                eprintln!("Error compiling {}: {}", input_path.to_str().unwrap(), e);
                std::process::exit(1);
            }
            Ok(Some(assembly_path)) => assembly_paths.push(assembly_path),
            _ => {}
        }
    }

    if args.compile_options.nolink {
        let mut failed = false;
        for assembly_path in assembly_paths {
            let output_path = assembly_path.with_extension("o");
            if !Command::new("gcc")
                .args(&[
                    "-c",
                    assembly_path.to_str().unwrap(),
                    "-o",
                    output_path.to_str().unwrap(),
                ])
                .status()?
                .success()
            {
                eprintln!(
                    "Error assembling {}: gcc failed",
                    assembly_path.to_str().unwrap()
                );
                failed = true;
            } else {
                println!("Object file written to: {}", output_path.to_str().unwrap());
            }

            if let Err(e) = remove_file(assembly_path) {
                eprintln!("Warning: failed to remove assembly file: {}", e);
            }
        }
        if failed {
            std::process::exit(1);
        }
    } else {
        let output_path = assembly_paths[0].with_extension("");

        let mut gcc_args = assembly_paths
            .iter()
            .map(|assembly_path| assembly_path.to_str().unwrap())
            .collect::<Vec<_>>();
        gcc_args.push("-o");
        gcc_args.push(output_path.to_str().unwrap());
        let success = Command::new("gcc").args(&gcc_args).status()?.success();

        if success {
            println!("Executable written to: {}", output_path.to_str().unwrap());
        }

        for assembly_path in assembly_paths {
            if let Err(e) = remove_file(assembly_path) {
                eprintln!("Warning: failed to remove assembly file: {}", e);
            }
        }

        if !success {
            eprintln!("Error linking: gcc failed");
            std::process::exit(1);
        }
    }

    Ok(())
}

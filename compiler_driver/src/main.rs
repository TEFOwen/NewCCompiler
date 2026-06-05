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
    #[clap(help = "Input C source file")]
    input: PathBuf,

    #[command(flatten)]
    compile_options: CompileOptions,
}

#[derive(Args)]
#[group(required = false, multiple = false)]
struct CompileOptions {
    #[clap(long, short, action, help = "Run only the lexer")]
    lex: bool,

    #[clap(long, short, action, help = "Run only the lexer and parser")]
    parse: bool,

    #[clap(
        long,
        short,
        action,
        help = "Run only TACKY intermediate representation generation"
    )]
    tacky: bool,

    #[clap(
        long,
        short,
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
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();

    let input_path = args.input;
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

    if args.compile_options.lex {
        println!("Tokens: {:#?}", tokens);
        return Ok(());
    }

    let ast = match compiler::parser::parse(tokens) {
        Ok(ast) => ast,
        Err(e) => {
            eprintln!("Parsing error: {}", e);
            std::process::exit(1);
        }
    };

    if args.compile_options.parse {
        println!("AST: {:#?}", ast);
        return Ok(());
    }

    let tacky = compiler::tacky::ToTacky::to_tacky(ast);

    if args.compile_options.tacky {
        println!("TACKY IR: {:#?}", tacky);
        return Ok(());
    }

    let assembly = compiler::codegen::to_assembly(tacky);

    if args.compile_options.codegen {
        println!("Assembly: {:#?}", assembly);
        return Ok(());
    }

    let assembly_path = input_path.with_extension("s");
    let assembly_file = File::create(&assembly_path)?;
    compiler::codeemission::EmitCode::emit_code(&assembly, assembly_file)?;

    if args.compile_options.assemble {
        println!("Assembly written to: {}", assembly_path.to_str().unwrap());
        return Ok(());
    }

    let output_path = input_path.with_extension("");
    assert!(
        Command::new("gcc")
            .args(&[
                assembly_path.to_str().unwrap(),
                "-o",
                output_path.to_str().unwrap(),
            ])
            .status()?
            .success()
    );

    if let Err(e) = remove_file(assembly_path) {
        eprintln!("Warning: failed to remove assembly file: {}", e);
    }

    println!("Executable written to: {}", output_path.to_str().unwrap());

    Ok(())
}

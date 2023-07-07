#!/usr/bin/env python3

import toml, os, argparse, sys, requests
from colorama import Fore, Style

def println(*values, prefix_style="::::", end='\n'):
    print(f"{Fore.GREEN + Style.BRIGHT}{prefix_style}{Style.RESET_ALL}", " ".join(values), end=end)

def eprintln(*values, prefix_style="::::", end='\n'):
    print(f"{Fore.RED + Style.BRIGHT}{prefix_style}{Style.RESET_ALL}", " ".join(values), end=end, file=sys.stderr)

def read(*values) -> str | None:
    println(*values)
    println(prefix_style="::", end='')
    try:
        return input()
    except (KeyboardInterrupt, EOFError):
        return None

def is_connected():
    # Source: https://stackoverflow.com/a/62078034 .
    url = 'http://www.google.com/'
    timeout = 5
    try:
        _ = requests.get(url, timeout=timeout)
        return True
    except requests.ConnectionError:
        return False

def update_version(cargo_path: list):
    println("Getting actual version...")
    vs = None
    for index, value in enumerate(cargo_path):
        with open(value, 'r') as file:
            data = toml.load(file)
        version: str = data['package']['version']
        version_splited = version.split(".")
        if args.nv:
            version_splited[0] = str(int(version_splited[0]) + 1)
            version_splited[1] = str(0)
            version_splited[2] = str(0)
        if args.fa:
            version_splited[1] = str(int(version_splited[1]) + 1)
            version_splited[2] = str(0)
        if args.ph:
            version_splited[2] = str(int(version_splited[2]) + 1)
        data['package']['version'] = ".".join(version_splited)
        with open("Cargo.toml", 'w') as file:
            toml.dump(data, file)
        if index == 0:
            vs = version_splited
    return vs

parser = argparse.ArgumentParser(prog="Uploader")
parser.add_argument("-n", "--new-version", action="store_true", dest="nv")
parser.add_argument("-fa", "--function-add", action="store_true", dest="fa")
parser.add_argument("-p", "--patch", action="store_true", dest="ph")
parser.add_argument("-se", "--skip-errors", action="store_true", dest="se")
parser.add_argument("-cp", "--cargo-path", nargs="*", dest="cp", default=["Cargo.toml"])
parser.add_argument("--dont-upload", action="store_true", dest="du")

args = parser.parse_args()

print(args)

version_splited = update_version(args.cp)

if args.se:
    println("Checking source...")
    wkd = os.getcwd()
    for x in args.cp:
        if not os.path.exists(x):
            eprintln("Cargo.toml not found!")
            sys.exit(1)
        os.chdir(os.path.split(x)[0])
        print(os.getcwd())
        code = os.system(f'cargo check')
        if code > 0:
            println("Fix the errors of the source")
            eprintln(f"The 'cargo check' command returned {code}")
            sys.exit(1)
    os.chdir(wkd)

println("Adding files to a new commit...")
os.system("git add .")
msg = read("Write a message for the commit:")
if msg is None:
    eprintln("Interrupted by user")
    sys.exit(1)
os.system(f'git commit -m "{".".join(version_splited)} {msg}"')

println("Checking internet connection...")
if is_connected():
    if not args.du:
        println("Uploading...")
        os.system("git push origin main")
    else:
        println("The next time you run this program, the commit will be uploaded")
else:
    println("The next time you run this program, and when you have internet connection, the commit will be uploaded")

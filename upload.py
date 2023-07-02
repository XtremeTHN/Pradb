#!/usr/bin/env python3

import toml, os, argparse, sys
from colorama import Fore, Style

def println(*values, prefix_style="::::", end='\n'):
    print(f"{Fore.GREEN + Style.BRIGHT}{prefix_style}{Style.RESET_ALL}", " ".join(values), end=end)

def eprintln(*values, prefix_style="::::", end='\n'):
    print(f"{Fore.RED + Style.BRIGHT}{prefix_style}{Style.RESET_ALL}", " ".join(values), end=end, file=sys.stderr)

def read(*values):
    println(*values)
    println(prefix_style="::", end='')
    return input()

def is_connected():
    return False

parser = argparse.ArgumentParser(prog="Uploader")
parser.add_argument("-n", "--new-version", action="store_true", dest="nv")
parser.add_argument("-fa", "--function-add", action="store_true", dest="fa")
parser.add_argument("-p", "--patch", action="store_true", dest="ph")
parser.add_argument("--dont-upload", action="store_true", dest="du")

args = parser.parse_args()

println("Getting actual version...")

with open("Cargo.toml", 'r') as file:
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

println("Checking source...")
code = os.system('cargo check')
if code > 0:
    println("Fix the errors of the source")
    eprintln(f"The 'cargo check' command returned {code}")

println("Adding files to a new commit...")
os.system("git add .")
msg = read("Write a message for the commit:")
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

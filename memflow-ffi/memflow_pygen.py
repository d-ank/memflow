#!/usr/bin/env python

"""
This script generates the wrapper code for memflow_py bindings
"""

import os
import platform
import sys
import subprocess
import pkg_resources
from argparse import ArgumentParser

required = {"pybindx", "pygccxml"}

for req in required:
    python = sys.executable
    os.system(python + " -m pip install --upgrade " + req)

from pybindx import CppWrapperGenerator

if __name__ == "__main__":
    clang_path = "clang"
    castxml_path = "castxml"

    if "Windows" in platform.system():
        # windows just aint gonna work lmao
        clang_path = "clang"
        castxml_path = "castxml"
        cflags = "-std=c++14 -w"
    else:
        os.system("chmod +x " + clang_path)
        os.system("chmod +x " + castxml_path)

    gen = CppWrapperGenerator(
        os.getcwd(),
        os.getcwd(),
        os.getcwd(),
        "castxml",
        os.getcwd() + "/memflow_py.yml",
        "clang",
        cflags,
        None,
    )
    gen.generate_wrapper()

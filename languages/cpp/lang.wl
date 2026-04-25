name="cpp"
desc="C++ project with cmake"
path="Code/C++"
direnv="use_cmake"
requires=["cmake", "g++"]
setup_priority="10"

build="cmake -B build && cmake --build build"
run="./build/main"
check="cmake --build build --target check"

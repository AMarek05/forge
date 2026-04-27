name="python"
desc="Python project with poetry"
path="Code/Python"
direnv="use_poetry"
requires=[]
setup_priority="10"

build="poetry build"
run="poetry run python"
test="poetry run pytest"
check="poetry run pyright"

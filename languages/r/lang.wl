name="r"
desc="R project with renv"
path="Code/R"
direnv="use_renv"
requires=[]
setup_priority="10"

build="R CMD build ."
run="Rscript run.R"
test="R CMD check ."
check="R CMD check ."

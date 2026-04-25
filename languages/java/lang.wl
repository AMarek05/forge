name="java"
desc="Java project with maven"
path="Code/Java"
direnv="use_maven"
requires=["maven", "java"]
setup_priority="10"

build="mvn package"
run="mvn exec:java"
check="mvn verify"

IMAGE_NAME=cargo-fuzz

info:
	$(info Available Targets)
	$(info --------------------------------------)
	$(info build    | build the docker image)
	
docker-rust:
	git clone https://github.com/octplane/docker-rust

build: docker-rust
	cd $< && docker build -t $< .
	docker build -t $(IMAGE_NAME) .

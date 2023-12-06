.PHONY: run
run: build-css
	@RUST_LOG=info cargo run

.PHONY: gencert
gencert:
	@mkcert localhost 127.0.0.1 ::1 ava.ai

.PHONY: build-css
build-css:
	@echo "Building CSS..."
	@npx tailwindcss build -i input.css -o ./public/css/main.css

.PHONY: watch
watch:
	@watchexec -e rs,js,css,html,j2 --ignore public -- make run

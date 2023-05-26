generate-artifacts:
	rm -rf ./tmp && mkdir tmp
	CARGO_TARGET_DIR=./tmp cargo build -p cheatcodes-actor --target wasm32-unknown-unknown --profile=wasm
	cp ./tmp/wasm32-unknown-unknown/wasm/cheatcodes_actor.wasm ./actors/artifacts/Cheatcodes.wasm
	rm -rf ./tmp
# This script takes care of testing your crate

set -ex

main() {
    cross build --target $TARGET --features vendored-openssl
    cross build --target $TARGET --release --features vendored-openssl

    if [ ! -z $DISABLE_TESTS ]; then
        return
    fi

    cross clippy --features vendored-openssl -- -D warnings
}

# we don't run the "test phase" when doing deploys
if [ -z $TRAVIS_TAG ]; then
    main
fi

build() {
    cargo build --target "$TARGET" --release --verbose
}

pack() {
    local tempdir
    local out_dir
    local package_name
    local gcc_prefix

    tempdir=$(mktemp -d 2>/dev/null || mktemp -d -t tmp)
    out_dir=$(pwd)
    package_name="$PROJECT_NAME-$TRAVIS_TAG-$TARGET"

    gcc_prefix=""

    # create a "staging" directory
    mkdir "$tempdir/$package_name"

    # copying the main binary
    cp "target/$TARGET/release/$BIN_NAME" "$tempdir/$package_name/"
    "${gcc_prefix}"strip "$tempdir/$package_name/$BIN_NAME"

    # readme and license
    cp README.md "$tempdir/$package_name"
    cp LICENSE-MIT "$tempdir/$package_name"
    cp LICENSE-APACHE "$tempdir/$package_name"

    # archiving
    pushd "$tempdir"
    tar czf "$out_dir/$package_name.tar.gz" "$package_name"/*
    popd
    rm -r "$tempdir"
}

make_deb() {
    local tempdir
    local architecture
    local version
    local dpkgname
    local conflictname
    local gcc_prefix

    case $TARGET in
        x86_64*)
            architecture=amd64
            gcc_prefix=""
            ;;
        i686*)
            architecture=i386
            gcc_prefix=""
            ;;
        *)
            echo "make_deb: skipping target '${TARGET}'" >&2
            return 0
            ;;
    esac
    version=${TRAVIS_TAG#v}
    if [[ $TARGET = *musl* ]]; then
      dpkgname=$PROJECT_NAME-musl
      conflictname=$PROJECT_NAME
    else
      dpkgname=$PROJECT_NAME
      conflictname=$PROJECT_NAME-musl
    fi

    tempdir=$(mktemp -d 2>/dev/null || mktemp -d -t tmp)

    # copy the main binary
    install -Dm755 "target/$TARGET/release/$BIN_NAME" "$tempdir/usr/bin/$BIN_NAME"
    "${gcc_prefix}"strip "$tempdir/usr/bin/$BIN_NAME"

    # readme and license
    install -Dm644 README.md "$tempdir/usr/share/doc/$PROJECT_NAME/README.md"
    install -Dm644 LICENSE-MIT "$tempdir/usr/share/doc/$PROJECT_NAME/LICENSE-MIT"
    install -Dm644 LICENSE-APACHE "$tempdir/usr/share/doc/$PROJECT_NAME/LICENSE-APACHE"

    # Control file
    mkdir "$tempdir/DEBIAN"
    cat > "$tempdir/DEBIAN/control" <<EOF
Package: $dpkgname
Version: $version
Section: utils
Priority: optional
Maintainer: Peter Ding <dfhayst@gmail.com>
Architecture: $architecture
Provides: $PROJECT_NAME
Conflicts: $conflictname
Description: Aget-rs - Fast Asynchronous Downloader.
EOF

    fakeroot dpkg-deb --build "$tempdir" "${dpkgname}_${version}_${architecture}.deb"
}


main() {
    build
    pack
    if [[ $TARGET = *linux* ]]; then
      make_deb
    fi
}

main

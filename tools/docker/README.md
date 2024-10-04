Build container:

```shell
docker build -t NAME:TAG .
```

Run container:

```shell
docker run -it --rm -v PATH_TO_PROJECT:/app:z NAME:TAG /bin/bash
```

Bench for `aarch64`:

```shell
cargo bench --target aarch64-unknown-linux-gnu
```

Bench for `x86_64`:

```shell
cargo bench --target x86_64-unknown-linux-gnu
```

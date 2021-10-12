# syntax=docker/dockerfile:1
# target (build)
FROM rustlang/rust:nightly

# set the working directory
WORKDIR /src

# copy the source to the directory
COPY . .

# build the source
RUN cargo build --release


# target (run)
FROM ubuntu:latest
RUN apt-get update
RUN apt-get install -y ca-certificates
#FROM alpine:latest
#RUN apk --no-cache add ca-certificates

# set the working directory
WORKDIR /bot

# copy the release
COPY --from=0 /src/target/release/kromer ./kromer

CMD ["./kromer"]

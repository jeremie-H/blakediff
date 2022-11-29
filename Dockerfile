#this docker file is just use to generate a binary to use on debian 11
#because building under ubuntu generate a binary which need GLIBC > 2.32 and 
#debian 11 use GLIBC 2.31

FROM rust:latest 
 
RUN apt update && apt upgrade -y 
 
WORKDIR /app 
 
CMD ["cargo", "build", "--release" ]

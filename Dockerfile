FROM ubuntu:22.04

RUN apt-get update
RUN apt-get install -y python3-pip
RUN apt-get install -y curl

RUN curl -LO https://freeshell.de/phd/chromium/jammy/pool/chromium_130.0.6723.58~linuxmint1+virginia/chromium_130.0.6723.58~linuxmint1+virginia_amd64.deb
RUN apt-get install -y ./chromium_130.0.6723.58~linuxmint1+virginia_amd64.deb
RUN apt-get install -y libasound2

WORKDIR /workdir
ENV PYTHONUNBUFFERED=1

COPY requirements.txt ./
RUN pip install -r requirements.txt

COPY scripts/ ./scripts/
COPY .env ./

CMD [ "python3", "scripts/twitter.py" ]

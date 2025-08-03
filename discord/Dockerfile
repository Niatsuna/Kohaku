FROM python:3.12.4-slim

WORKDIR /kohaku

COPY . .

RUN apt-get update && apt-get upgrade -y

CMD ["python", "main.py"]
FROM python:3.12-slim

WORKDIR /kohaku
COPY . .

RUN apt-get update && \
	apt-get upgrade -y && \
	apt-get clean && \
	rm -rf /var/lib/apt/lists/*

CMD ["python", "main.py"]
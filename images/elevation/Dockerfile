FROM ghcr.io/waynevanson/open-elevation:main

COPY ./images/elevation/config.ini /code/data/

VOLUME /data

EXPOSE 8080

ENTRYPOINT ["stdbuf", "-i0", "-o0", "-e0", "python3", "server.py"]
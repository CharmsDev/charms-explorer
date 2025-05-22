FROM postgres:15

# Copy initialization scripts
COPY init/ /docker-entrypoint-initdb.d/

# Create a subdirectory for PostgreSQL data
RUN mkdir -p /var/lib/postgresql/data/pgdata && \
    chown -R postgres:postgres /var/lib/postgresql/data/pgdata && \
    chmod 700 /var/lib/postgresql/data/pgdata

# Set the data directory
ENV PGDATA=/var/lib/postgresql/data/pgdata

# Expose PostgreSQL port
EXPOSE 5432

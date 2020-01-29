description "Biome Supervisor"

start on filesystem or runlevel [2345]

script
    export RUST_LOG=${log_level}
    export HAB_STATS_ADDR=localhost:8125
%{ for feature in enabled_features ~}
    export HAB_FEAT_${upper(feature)}=1
%{ endfor ~}
    export SSL_CERT_FILE=$(bio pkg path core/cacerts)/ssl/cert.pem
    echo $$ > /var/run/bio-sup.pid
    echo "starting bio sup with: ${flags}" >> /var/log/bio-sup.log
    exec /bin/bio run ${flags} >> /var/log/bio-sup.log
end script

pre-start script
    echo "[`date`] bio-sup service starting" >> /var/log/bio-sup.log
end script

pre-stop script
    rm /var/run/bio-sup.pid
    echo "[`date`] bio-sup service stopping" >> /var/log/bio-sup.log
end script

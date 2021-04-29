const express = require('express');
const cors = require('cors');
const path = require('path');
const app = express();
const port = process.env.PORT || 3000;

app.use(cors());
app.use(express.static(__dirname));
app.use(express.static(path.join(__dirname, 'build')));

const dgram = require('dgram');
const socket = dgram.createSocket('udp4');

const Denque = require('denque');
const requestQ = new Denque();
let reqno = 0;

let hosts = [];
let hostIdx = 0;
let foundLeader = false;
let logData = []; 
let startIdx = [];

const fs = require('fs');
fs.readFile('./hosts.txt', (err, data) => {
    if (err) throw err;
    data = data.toString('utf8').trim().split('\n');
    data.forEach(d => {
        d = d.split(':');
        hosts.push({'addr':d[0], 'port':d[1]});
        logData.push([]);
        startIdx.push(0);
    });

    function isRequestLogResponse(msg) {
        return msg['RequestLogResponse'] != undefined;
    }

    function isClientResponse(msg) {
        return msg['ClientResponse'] != undefined;
    }

    socket.on('message', (msg, rinfo) => {
        let obj = JSON.parse(msg);
        if (isRequestLogResponse(obj)) {
            let host = {'addr':rinfo.address, 'port':`${rinfo.port}`};
            let idx = hosts.findIndex(h => JSON.stringify(h) == JSON.stringify(host));
            let entries = obj['RequestLogResponse']['entries'];
            for (var i in entries) {
                let e = entries[i];
                if (e['applied'] == true) {
                    logData[idx].push(e);
                    startIdx[idx]++;
                }
            }
            // startIdx[idx] = obj['RequestLogResponse']['applied_idx'] + 1;
        } else if (isClientResponse(obj)) {
            if (obj['ClientResponse']['success'] == true && obj['ClientResponse']['opid'] == requestQ.peekFront()) {
                foundLeader = true;
                requestQ.shift();
            } else {
                foundLeader = false;
            }
        } else {
            console.log(`Unknown message received: ${msg}`);
        }
    });

    socket.on('listening', () => {
        console.log('socket listening');
        setInterval(() => {
            for (let idx = 0; idx < hosts.length; idx++) {
                socket.send(`{"RequestLog":{"start_idx":${startIdx[idx]}}}`, hosts[idx]['port'], hosts[idx]['addr']);
            }
        }, 250);

        setInterval(() => {
            if (!requestQ.isEmpty()) {
                if (!foundLeader) {
                    hostIdx++;
                    hostIdx %= hosts.length;
                }
                socket.send(`{"ClientRequest":{"opid":${requestQ.peekFront()}}}`, hosts[hostIdx]['port'], hosts[hostIdx]['addr']);
            }
        }, 100);
    });

    socket.on('error', (err) => {
        console.log(`socket error: ${err}`);
        socket.close();
    });

    socket.bind(5800);

    app.get('/', (req, res) => {
        res.sendFile(path.join(__dirname, 'build', 'index.html'));
    });

    app.get('/state', (req, res) => {
        res.send(logData);
    });

    app.post('/request', (req, res) => {
        console.log('making request');
        reqno++;
        requestQ.push(reqno);
        res.send('request sent');
    });

    app.listen(port, () => {
        console.log(`listening at localhost:${port}`);
    });
});

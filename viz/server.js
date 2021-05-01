const express = require('express');
const favicon = require('express-favicon');
const cors = require('cors');
const path = require('path');
const app = express();
const port = process.env.PORT || 3000;

app.use(cors());
app.use(express.static(__dirname));
app.use(express.static(path.join(__dirname, 'build')));
app.use(favicon(__dirname + '/build/favicon.ico'));

const dgram = require('dgram');
const socket = dgram.createSocket('udp4');

const Denque = require('denque');
const requestQ = new Denque();
let reqno = 0;

let hosts = [];
let hostIdx = 0;
let foundLeader = false;
let logData = []; 
let hostStatus = [];
let hostTimer = [];
let startIdx = [];

const fs = require('fs');
fs.readFile('./hosts.txt', (err, data) => {
    if (err) throw err;
    data = data.toString('utf8').trim().split('\n');
    data.forEach(d => {
        d = d.split(':');
        hosts.push({'addr':d[0], 'port':d[1]});
        logData.push([]);
        hostStatus.push('Follower');
        hostTimer.push(0);
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
        let host = {'addr':rinfo.address, 'port':`${rinfo.port}`};
        let idx = hosts.findIndex(h => JSON.stringify(h) == JSON.stringify(host));
        hostTimer[idx] = 0;
        if (hostStatus[idx] == 'Crashed') hostStatus[idx] = 'Follower';
        if (isRequestLogResponse(obj)) {
            let entries = obj['RequestLogResponse']['entries'];
            for (var i in entries) {
                let e = entries[i];
                if (e['applied'] == true) {
                    logData[idx].push(e);
                    startIdx[idx]++;
                    reqno = Math.max(reqno, e['opid']);
                }
            }
        } else if (isClientResponse(obj)) {
            if (obj['ClientResponse']['success'] == true && obj['ClientResponse']['opid'] == requestQ.peekFront()) {
                foundLeader = true;
                hostStatus[idx] = 'Leader';
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

        setInterval(() => {
            for (var i in hosts) {
                hostTimer[i]++;
                if (hostTimer[i] == 100) {
                    hostStatus[i] = 'Crashed';
                }
            }
        }, 10);
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
        res.send({logs: logData, statuses: hostStatus});
    });

    app.post('/request', (req, res) => {
        reqno++;
        requestQ.push(reqno);
        res.send('request sent');
    });

    app.get('/refresh', (req, res) => {
        for (var i in hosts) {
            logData[i] = [];
            hostStatus[i] = 'Follower';
            hostTimer[i] = 0;
            startIdx[i] = 0;
        }
        reqno = 0;
        res.redirect('/');
    });

    app.listen(port, () => {
        console.log(`listening at localhost:${port}`);
    });
});

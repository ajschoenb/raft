const dgram = require('dgram');
const socket = dgram.createSocket('udp4');

let host_addr = "18.190.157.184";
let host_port = 5800;

socket.on('message', (msg, rinfo) => {
    console.log(`server got: ${msg} from ${rinfo.address}:${rinfo.port}`);
});

socket.on('listening', () => {
    console.log('server listening');
    setInterval(() => {
        socket.send('{"RequestLog":{"start_idx":0}}', host_port, host_addr);
    }, 1000);
});

socket.bind(5885);

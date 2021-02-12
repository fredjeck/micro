var sk = new WebSocket('ws://127.0.0.1:4200/uplink');

// Connection opened
sk.addEventListener('open', function (event) {
    sk.send('Hello Server!');
});

// Listen for messages
sk.addEventListener('message', function (event) {
    console.log('Message from server ', event.data);
    if (event.data !== 'Pong !') {
        document.location = document.location.origin + '/' + event.data;
    }
});

sk.send("fart");
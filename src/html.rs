pub(crate) const INDEX: &str = r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <title>server_clipboard</title>
    <meta charset="utf-8"/>
    <style>
        li {
            float: left;
            width: 33%;
        }
    </style>
    <script>
        function s() {
            let k = key.value
            let v = val.value
            var xhr = new XMLHttpRequest();
            xhr.open("POST", "/" + k, true);
            xhr.onreadystatechange = function () {
                if (this.readyState != 4) return;

                if (this.status == 200) {
                    location.reload()
                } else {
                    alert("err")
                }
            };
            xhr.send(v)
        }
    </script>
</head>
<body>
<div>
    key: <input id="key">
    value: <textarea id="val"></textarea>
    <button onclick=s()>submit</button>
</div>
</body>
</html>
"#;

pub(crate) const HELP: &str = r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <title>help</title>
    <style>
        table td {
            border: 1px solid;
        }
    </style>
</head>
<body>
<table>
    <thead>
    <td style="width: 20%">key</td>
    <td style="width: 70%">description</td>
    </thead>

    <tr>
        <td>store key, value</td>
        <td><code>curl -X POST -d "$value" $host/$key </code></td>
    </tr>
    <tr>
        <td>get by key</td>
        <td><code> curl $host/$key?quiet</code></td>
    </tr>
    <tr>
        <td>times optional int</td>
        <td> The number of times that can be obtained, Default 1 time</td>
    </tr>
    <tr>
        <td>minutes optional int</td>
        <td> Saved minutes, Default 1 minute</td>
    </tr>
    <tr>
        <td>private optional any</td>
        <td>Whether to display on the home page list, store page only</td>
    </tr>
    <tr>
        <td>quiet optional any</td>
        <td>Passing this parameter will close websocket function, get page only, for curl</td>
    </tr>
    <tr>
        <td>demo</td>
        <td><code>curl -X POST -d "abcdef" "localhost:7259/abc?times=2&private"</code></td>
    </tr>
    <tr>
        <td>websocket</td>
        <td>Get page support websocket .Click to write or read the clipboard, must be https</td>
    </tr>
    <tr>
        <td></td>
        <td>
times minutes .These two parameters will take effect when the first one enters the websocket room</td>
    </tr>
</table>
</body>
</html>
"#;

pub(crate) const GET: &str = r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <title>get</title>
    <meta charset="utf-8"/>
    <script>
        'use strict'

        const pathname = window.location.pathname
        const queryString = window.location.search
        const joinString = "/join " + pathname + " " + queryString
        var messageArray = []
        let needConfirm = false
        let wsData = null
        window.onload = () => {
            let conn = null
            const set_text = (wsData) => {
                if (null != wsData.message) {
                    clipboard_text.innerHTML = wsData.message
                }
                if (wsData.times) {
                    times.innerHTML = wsData.times
                }
                if (null != wsData.minutes) {
                    minutes.innerHTML = wsData.minutes
                }
                if (wsData.total) {
                    total.innerHTML = wsData.total
                }
                if (wsData.result) {
                    result.innerHTML = wsData.result
                }
                if (null != wsData.remaining) {
                    remaining.innerHTML = wsData.remaining
                }
            }

            const do_send_receive = () => {
                if (!conn) return
                let txt = clipboard_text.innerHTML
                if (!txt) {
                    navigator.clipboard.readText().then(
                        clipText => {
                            console.log(clipText)
                            conn.send(clipText)
                        }
                    )
                } else if (needConfirm) {
                    if (confirm("Are you sure to overwrite your clipboard?")){
                        needConfirm = false
                        result.innerHTML = "click to write clipboard"
                    }
                }else{
                    navigator.clipboard.writeText(txt).then(() => {
                        if (messageArray.length === 0) {
                            clipboard_text.innerHTML = ""
                            result.innerHTML = ""
                            needConfirm = false
                        } else {
                            clipboard_text.innerHTML = messageArray.shift()
                            queue.innerHTML = messageArray.length
                            result.innerHTML = "click to confirm"
                            needConfirm = true
                        }

                    })
                }

            }
            const connect = () => {
                const wsUri =
                    (window.location.protocol === 'https:' ? 'wss://' : 'ws://') +
                    window.location.host +
                    '/ws/'
                conn = new WebSocket(wsUri)
                console.log('Connecting...')
                conn.onopen = function () {
                    console.log(joinString)
                    conn.send(joinString)
                }
                conn.onmessage = function (e) {
                    console.log('Received: ' + e.data)

                    wsData = JSON.parse(e.data)
                    if ("" === clipboard_text.innerHTML || "" === times.innerHTML ) {
                        set_text(wsData)
                    } else {
                        messageArray.push(wsData.message)
                        queue.innerHTML = messageArray.length
                    }

                }
                conn.onclose = function () {
                    console.log('Disconnected.')
                    conn = null
                    setTimeout(() => connect(), 8000)
                }
            }
            connect()
            window.onclick = () => {
                do_send_receive()
            }
            window.onkeyup = (e) => {
                if (e.key === 'Enter') {
                    do_send_receive()
                }
            }
        }
    </script>
    <style>
        table td {
            border: 1px solid;
        }

        td {
            width: 20%;
        }
    </style>
</head>

<body>
<table>
    <tr>
        <td>times</td>
        <td id="times"></td>
        <td>minutes</td>
        <td id="minutes"></td>

    </tr>

    <tr>
        <td>remaining</td>
        <td id="remaining"></td>
        <td>people in the room</td>
        <td id="total"></td>
    </tr>
    <tr>
        <td>result or hint</td>
        <td id="result"></td>
         <td>number in the queue</td>
        <td id="queue"></td>
    </tr>
    <tr>
        <td>text</td>
        <td colspan="3" id="clipboard_text">{{}}</td>
    </tr>
</table>
</body>
</html>
"#;

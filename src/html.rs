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
</head>
<body>
    <ol>
        <li> save k,v<br><code>curl -X POST -d "[value]" [server]/[key] </code></li>
        <li> get k <br><code> curl [server]/[key]?quiet</code></li>
        <li>
            The number of times that can be obtained Default 11 <br> optional times int
        </li>
        <li>
            Saved minutes Default 1 minute <br> optional minutes int
        </li>
        <li>
            Whether to display on the home page list <br> optional private any
        </li>
        <li> demo <br> curl -X POST -d "abcdefg" "localhost:7259/abc?times=2&private"</li>
        <li> Get page support websocket .Click to write or read the clipboard.
        </li>
    </ol>
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
                if (wsData.message) {
                    clipboard_text.innerHTML = wsData.message
                }
                if (wsData.times) {
                    times.innerHTML = wsData.times
                }
                if (wsData.minutes) {
                    minutes.innerHTML = wsData.minutes
                }
                if (wsData.total) {
                    total.innerHTML = wsData.total
                }
                if (wsData.result) {
                    result.innerHTML = wsData.result
                }
                if (wsData.remaining !== null) {
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
                    if (clipboard_text.innerHTML === "") {
                        set_text(wsData)
                    } else {
                        messageArray.push(wsData.message)
                        queue.innerHTML = messageArray.length
                    }

                }
                conn.onclose = function () {
                    console.log('Disconnected.')
                    conn = null
                    setTimeout(() => connect(), 5000)
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

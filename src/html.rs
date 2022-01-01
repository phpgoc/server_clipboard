pub(crate) const INDEX: &str = r#"
<div>
    key: <input id="k">
    value: <textarea  id="v"></textarea>
    <button onclick=s()>submit</button>
</div>
<script>
function s(){
    let k = (document.getElementById("k").value)
    let v = (document.getElementById("v").value)
    var xhr = new XMLHttpRequest();
    xhr.open("POST", "/"+k, true);
    xhr.onreadystatechange = function () {
        if (this.readyState != 4) return;

        if (this.status == 200) {
            location.reload()
        }else{
            alert("err")
        }
    };
    xhr.send(v)
}
</script>
    "#;

pub(crate) const HELP: &str = r#"
<ol>
    <li> 存入k,v<br><code>curl -X POST -d "[value]" [server]/[key] </code></li>
    <li> 获取k <br><code> curl [server]/[key]</code></li>
    <li> 可以获取的次数默认1 <br> 可选项  times int  </li>
    <li> 保存的分钟 默认1分钟 <br> 可选项 minutes int </li>
    <li> 是否在首页列表显示 <br> 可选项 private 任意string </li>
    <li> demo <br> curl -X POST -d "abcdefg" "localhost:7259/abc?times=2&private=a" </li>
</ol>
"#;

pub(crate) const GET: &str = r#"<!DOCTYPE html>
<html>
  <head>
    <title>get</title>
    <meta charset="utf-8" />
    <script>
      'use strict'

      const pathname = window.location.pathname
      const queryString = window.location.search
      const joinString = "/join " + pathname + " " + queryString
      window.onload = () => {
        let conn = null
        const do_send_receive =  () => {
            if (!conn) return
            let txt = clipboard_text.innerHTML
            if(!txt){
                navigator.clipboard.readText().then(
                    clipText => {
                        console.log(clipText)
                        conn.send(clipText)
                    }
                )
            }else{
                navigator.clipboard.writeText(txt).then(
                    clipboard_text.innerHTML = ""
                )
            }

        }
        const connect = () => {
          const wsUri =
           'ws://' +
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
            clipboard_text.innerHTML =  e.data
          }
          conn.onclose = function () {
            console.log('Disconnected.')
            conn = null
            setTimeout( ()=> connect(), 5000)
          }
        }
        connect()
        window.onclick = () =>{
            do_send_receive()
        }
        window.onkeyup = (e) => {
          if (e.key === 'Enter') {
            do_send_receive()
          }
        }
      }
    </script>
  </head>

  <body>
       <span id="clipboard_text">{{}}</span>
  </body>
</html>
"#;


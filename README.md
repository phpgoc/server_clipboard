## 存入k,v<br>
``
curl -X POST -d "[value]" [HOST]/[key] 
``

## 获取k 
``
curl [HOST]/[key]
``
## 参数，
### POST和 websocket的第一进入房间的人有效
- 可以获取的次数默认1 可选项  times int 
- 保存的分钟 默认1分钟  可选项 minutes int 
- 是否在首页列表显示 可选项 private 任意string ，websocket都是public的

## demo 

``
curl -X POST -d "abcdefg" "localhost:7259/abc?times=2&private=a" 
``
## get页面支持websocket,鼠标单击就会write 或 read 剪贴板


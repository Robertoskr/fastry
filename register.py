import json as jsonParser
import time 


class HTTPResponse: 
    def __init__(self, code: int, type: str, body: str, headers: str): 
        self.code = code
        self.type = type 
        self.body = body
        self.headers = headers

class JSONResponse(HTTPResponse): 
    def __init__(
        self, 
        code: int = 200, 
        json: dict = {}, 
        headers: dict = {}
    ): 
        super().__init__(code=code, type="application/json", body=jsonParser.dumps(json), headers=jsonParser.dumps(headers))

#->r /
def route(application, request):
    time.sleep(1) 
    return JSONResponse(json={"hola": "adios"})

#->r /hola/<something>/adios
def hola(application, request): 
    return HTTPResponse(200, "text/html", "<h1 style='color:blue'>HOla</h1>", headers={})


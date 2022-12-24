import json as jsonParser 


class FastryApplication: 
    """
    Object containing all the info of your application!,
    its going to be passed as a parameter to your handlers.
    Is going to be called to initialize at the start of the application
    here you can store db connections, redis connections etc, so you don't need 
    to open new connections every time, that you receive a new request.
    """

    def __init__(self): 
        # initialize your application here
        self.something = 1
        pass


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



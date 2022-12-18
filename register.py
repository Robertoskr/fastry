
class HTTPResponse: 
    def __init__(self, code: int, type: str, body: str): 
        self.code = code
        self.type = type 
        self.body = body

    def to_dict(self) -> dict: 
        return {
            "code": self.code, 
            "type": self.type, 
            "body": self.body,
        } 

#->r /home
def route(application, request):
    return HTTPResponse(200, "application/json", '{"hola": "adios"}') 


#->r /somenice/others
def other_route(application, request):
    print(application, request)


#->r /some/route/hola
def hola_handler(application, request):
    print(application, request)

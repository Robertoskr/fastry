import time
import os
from fastry import FastryApplication, HTTPResponse, JSONResponse

#->r /
def route(application: FastryApplication, request: dict):
    return JSONResponse(json={"hola": "adios"})

#->r /hola/<something>/<adios>/<hola>/hola
def hola(application: FastryApplication, request: dict): 
    return HTTPResponse(200, "text/html", "<h1 style='color:blue'>HOla</h1>", headers={})


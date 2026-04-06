<img width="577" height="100" alt="image" src="https://github.com/user-attachments/assets/25d09ab8-ef22-4f94-a378-31489c22905b" />

<hr>

RustPOS is a simple Point of Sale system written in Rust. It is very easy to get going, does not require any additional setup and consists of a REST API backend and a web frontend using webassembly. 
This is a simple implementation using modern technologies. It was born out of my frustration with free and open source POS software often being very brittle and relying on heavyweight technology stacks.

> [!NOTE]
> This project is a functional proof of concept and not intended to be used in a production environment.
> For example, the backend does not use any form of authentication yet. This project also likely does not meet the requirements of your fiscal authorities.

<img width="1181" height="926" alt="image" src="https://github.com/user-attachments/assets/47ed4d5f-25cf-4da2-951e-4404e58e518b" />


## Features

* All-Rust solution
* Leptos and Webassembly powered web UI, no javascript
* Simple REST API
* SQLite database
* Completely configurable categories and items for sale via web UI
* Supports running tabs
* Change calculation
* Sales report generation: day, month, and custom date range reports
* POS printer support


<img width="1180" height="590" alt="image" src="https://github.com/user-attachments/assets/593e7a9a-544b-419e-be8a-df64b5fdfb8f" />

<img width="1179" height="1064" alt="image" src="https://github.com/user-attachments/assets/839da9a7-8090-42a3-b1cd-dec99cea7302" />


## Feature wishlist

* Kitchen printer support
* User roles: Admin, Sales, Reporting
* Inventory tracking

## Setup

### Install prerequisites
```
# Install Trunk (build tool for Rust WASM apps)
cargo install trunk

# Install WASM target
rustup target add wasm32-unknown-unknown
```


### Production Deployment

NOTE: This is a proof of concept and not ready for production use.

The build script ```build.sh``` will build the application and copy all the neccessary files into the "rustpos" subdirectory. 
You can copy that directory anywhere you want for a more permanent installation.

```
# Build application
./build.sh

# Run application
cd rustpos
./rustpos
```

### Receipt Printer Support

RustPOS will enumerate all receipt printers connected via serial port or USB, and use the first one it finds. Obviously this won't work for all setups, but for this proof of concept it should suffice.
The printout is designed for 80mm receipt printers and have been tested using a Munbyn ITPP098 connected via USB.

<img width="713" height="867" alt="image" src="https://github.com/user-attachments/assets/9334a0c5-aefa-4bfa-b5c0-d5df42c33415" />


### Customization

It is possible to change the logo used on the web UI as well as on the receipt printouts. The images can be found in ```frontend/assets/```

# Persistence

The data is stored in a sqlite database under "data/"


# Elevated Cycling API

You provide a coordinate and a radius to search, and receive the sickest down hill circuit of your life.

# Usage

Work in progress, this doesn't work very well.

- Still needs a webpage for [MapBBCode](http://mapbbcode.org/) to give.
- An algorithm that isn't trash.

This should give you a circuit if you have 2 entry points to your street, otherwise it'll probably fail at the moment.

## Self Hosted

Currenlty run as a dev environment with file watching servers because I don't plan to deploy this into the cloud
(unless I want to lose all my money).

1. Install and setup prerequisites
   - docker ^24.1.7
   - docker-compose ^2.23.3
2. Clone repository locally.
3. From the root of the repository, run the `docker-compose up`.
4. Run the following command, subsituting values in the body with your parameters:


Radius is in KM.

```sh
curl --request=GET --header='ContentType: application/json' http://localhost:3000/api/circuit/downhill \
--data='{ "longitude": 0.0, "latitude": 0.0, "radius": 0.5}'
```
